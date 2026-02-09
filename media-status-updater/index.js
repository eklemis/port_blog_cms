// index.js
const functions = require("@google-cloud/functions-framework");
const { Storage } = require("@google-cloud/storage");
const { Pool } = require("pg");

const storage = new Storage();

// Bucket configuration - variants are stored separately from uploads
const READY_BUCKET = "blogport-cms-ready";

// Initialize PostgreSQL connection pool
const pool = new Pool({
  connectionString: process.env.DATABASE_URL,
  ssl:
    process.env.NODE_ENV === "production"
      ? { rejectUnauthorized: false }
      : false,
  max: 10,
  idleTimeoutMillis: 30000,
  connectionTimeoutMillis: 10000,
});

// Graceful shutdown
process.on("SIGTERM", async () => {
  console.log("SIGTERM received, closing pool...");
  await pool.end();
});

/**
 * Map variant size to semantic variant_type
 */
function getVariantType(size) {
  const sizeMap = {
    150: "thumbnail",
    320: "small",
    768: "medium",
    1200: "large",
  };
  return sizeMap[size] || `size_${size}`;
}

/**
 * Determine MIME type from file path
 */
function getMimeTypeFromPath(path) {
  const ext = path.split(".").pop().toLowerCase();
  const mimeTypes = {
    webp: "image/webp",
    jpg: "image/jpeg",
    jpeg: "image/jpeg",
    png: "image/png",
    gif: "image/gif",
    avif: "image/avif",
  };
  return mimeTypes[ext] || "application/octet-stream";
}

/**
 * Insert media variants with conflict resolution
 * All data comes from the manifest - no GCS calls needed
 */
async function insertVariants(client, mediaId, variantsData) {
  if (!variantsData || variantsData.length === 0) {
    console.log("No variants to insert");
    return;
  }

  console.log(
    `Inserting ${variantsData.length} variants (bucket: ${READY_BUCKET})`,
  );

  for (const variant of variantsData) {
    const variantType = getVariantType(variant.size);
    const mimeType = getMimeTypeFromPath(variant.path);

    try {
      // Upsert variant: insert if not exists, update if exists (idempotent)
      await client.query(
        `
        INSERT INTO media_variants (
          media_id,
          variant_type,
          bucket_name,
          object_key,
          mime_type,
          file_size_bytes,
          width,
          height
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (media_id, variant_type)
        DO UPDATE SET
          bucket_name = EXCLUDED.bucket_name,
          object_key = EXCLUDED.object_key,
          mime_type = EXCLUDED.mime_type,
          file_size_bytes = EXCLUDED.file_size_bytes,
          width = EXCLUDED.width,
          height = EXCLUDED.height,
          created_at = CASE
            WHEN media_variants.bucket_name = EXCLUDED.bucket_name
              AND media_variants.object_key = EXCLUDED.object_key
            THEN media_variants.created_at
            ELSE CURRENT_TIMESTAMP
          END
        RETURNING id, variant_type
        `,
        [
          mediaId,
          variantType,
          READY_BUCKET,
          variant.path,
          mimeType,
          variant.file_size_bytes || 0, // From manifest
          variant.width, // From manifest
          variant.height, // From manifest
        ],
      );

      const sizeKB = variant.file_size_bytes
        ? (variant.file_size_bytes / 1024).toFixed(1)
        : "unknown";
      console.log(
        `  ✓ Variant '${variantType}': ${variant.width}x${variant.height} (${sizeKB} KB)`,
      );
    } catch (err) {
      console.error(
        `  ✗ Failed to insert variant '${variantType}':`,
        err.message,
      );
      // Continue with other variants even if one fails
    }
  }
}

functions.cloudEvent("updateMediaRecord", async (cloudEvent) => {
  const file = cloudEvent.data;
  const fileName = file.name;
  const bucketName = file.bucket;

  // Pattern matching: only process manifest.json files
  if (!fileName.endsWith("/manifest.json") && fileName !== "manifest.json") {
    console.log(`Skipping: ${fileName} does not match manifest pattern.`);
    return;
  }

  console.log(`Processing manifest: gs://${bucketName}/${fileName}`);

  let client;
  try {
    // Download and parse manifest
    const fileBuffer = await storage
      .bucket(bucketName)
      .file(fileName)
      .download();
    const manifest = JSON.parse(fileBuffer.toString());

    const { media_id, state, updated_at, variants } = manifest;

    // Validate required fields
    if (!media_id || !state || !updated_at) {
      console.error("Invalid manifest: missing required fields", manifest);
      return;
    }

    // Validate state enum
    const validStates = ["pending", "processing", "ready", "failed"];
    if (!validStates.includes(state)) {
      console.error(
        `Invalid state: ${state}. Must be one of: ${validStates.join(", ")}`,
      );
      return;
    }

    console.log("--- Manifest Extracted ---");
    console.log(`Media ID:   ${media_id}`);
    console.log(`State:      ${state}`);
    console.log(`Updated At: ${updated_at}`);
    if (state === "ready" && variants) {
      console.log(`Variants:   ${variants.length}`);
    }

    // Get database connection
    client = await pool.connect();

    // Start transaction for atomicity
    await client.query("BEGIN");

    try {
      // Update media status (idempotent with ordering protection)
      const result = await client.query(
        `
        UPDATE media
        SET
          status = $1::media_status,
          updated_at = $2
        WHERE
          id = $3
          AND deleted_at IS NULL
          AND status NOT IN ('ready', 'failed')
          AND updated_at <= $2
        RETURNING id, status, updated_at
        `,
        [state, updated_at, media_id],
      );

      if (result.rowCount === 0) {
        // Check why update didn't happen
        const checkResult = await client.query(
          `SELECT id, status, updated_at, deleted_at FROM media WHERE id = $1`,
          [media_id],
        );

        if (checkResult.rowCount === 0) {
          console.log(`Media ID ${media_id} not found in database`);
          await client.query("ROLLBACK");
          return;
        }

        const current = checkResult.rows[0];
        if (current.deleted_at) {
          console.log(`Media ID ${media_id} is soft-deleted, skipping update`);
        } else if (["ready", "failed"].includes(current.status)) {
          console.log(
            `Media ID ${media_id} is in terminal state '${current.status}'`,
          );

          // Even if status wasn't updated, insert variants if state is 'ready'
          // This handles re-processing scenarios
          if (state === "ready" && variants && variants.length > 0) {
            console.log(
              "Terminal state reached, but inserting/updating variants...",
            );
            await insertVariants(client, media_id, variants);
          }
        } else if (new Date(current.updated_at) > new Date(updated_at)) {
          console.log(
            `Media ID ${media_id} has newer timestamp (${current.updated_at} > ${updated_at}), skipping update`,
          );
        }

        await client.query("COMMIT");
        console.log("--------------------------");
        return;
      }

      const updated = result.rows[0];
      console.log(
        `✓ Successfully updated media ${updated.id} to state '${updated.status}'`,
      );

      // If state is 'ready', insert variants
      if (state === "ready" && variants && variants.length > 0) {
        await insertVariants(client, media_id, variants);
      }

      await client.query("COMMIT");
      console.log("--------------------------");
    } catch (err) {
      await client.query("ROLLBACK");
      throw err;
    }
  } catch (err) {
    console.error(`Error processing ${fileName}:`, err.message);
    console.error(err.stack);
  } finally {
    if (client) {
      client.release();
    }
  }
});
