const functions = require("@google-cloud/functions-framework");
const { Storage } = require("@google-cloud/storage");

const storage = new Storage();

functions.cloudEvent("updateMediaRecord", async (cloudEvent) => {
  // 1. Extract file metadata from the event
  const file = cloudEvent.data;
  const fileName = file.name; // e.g., "folder1/subfolder/manifest.json"
  const bucketName = file.bucket;

  // 2. Pattern Matching: Only process if it ends in 'manifest.json'
  if (!fileName.endsWith("/manifest.json") && fileName !== "manifest.json") {
    console.log(`Skipping: ${fileName} does not match manifest pattern.`);
    return;
  }

  console.log(`Processing manifest: gs://${bucketName}/${fileName}`);

  try {
    // 3. Download and read the file content
    const fileBuffer = await storage
      .bucket(bucketName)
      .file(fileName)
      .download();
    const manifest = JSON.parse(fileBuffer.toString());

    // 4. Extract your specific fields
    const { media_id, state, updated_at } = manifest;

    // 5. Log for now (Postgres update comes next!)
    console.log("--- Manifest Extracted ---");
    console.log(`Media ID:   ${media_id}`);
    console.log(`State:      ${state}`);
    console.log(`Updated At: ${updated_at}`);
    console.log("--------------------------");
  } catch (err) {
    console.error(`Error processing ${fileName}:`, err.message);
  }
});
