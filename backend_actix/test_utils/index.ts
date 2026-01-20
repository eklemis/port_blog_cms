import express from "express";
import jwt, { Algorithm } from "jsonwebtoken";
import { randomUUID } from "crypto";
import { pool } from "./db";

/**
 * 🚨 SAFETY GUARD
 * This helper must NEVER run in production.
 */
if (process.env.NODE_ENV === "production") {
  throw new Error("JWT test helper must not run in production");
}

const app = express();

app.get("/health", (_req, res) => {
  if (process.env.NODE_ENV === "production") {
    console.error("🚨 JWT test helper running in production environment");
    return res.status(500).json({
      status: "error",
      reason: "test-helper-running-in-production",
    });
  }

  res.status(200).json({
    status: "ok",
    environment: process.env.NODE_ENV ?? "undefined",
  });
});

/**
 * MUST match Rust JwtConfig.secret_key exactly
 */
const VALID_SECRET = process.env.JWT_SECRET ?? "test-secret";

/**
 * Intentionally wrong secret for InvalidSignature
 */
const INVALID_SECRET = "wrong-secret";

/**
 * Rust-compatible JWT claims
 */
type JwtClaims = {
  sub: string; // UUID
  exp: number; // unix timestamp
  iat: number; // unix timestamp
  nbf: number; // unix timestamp
  token_type: "access";
  is_verified: boolean;
};

type TokenKind =
  | "Valid"
  | "Expired"
  | "NotYetValid"
  | "InvalidSignature"
  | "Malformed";

/**
 * Unix timestamp helper
 */
const now = (): number => Math.floor(Date.now() / 1000);

app.get("/token/access/:token_kind/:user_id", (req, res) => {
  const tokenKind = req.params.token_kind as TokenKind;
  const userId = req.params.user_id;
  const isVerified = req.query.is_verified === "true";

  // Basic UUID sanity check
  if (!/^[0-9a-fA-F-]{36}$/.test(userId)) {
    return res.status(400).json({ error: "Invalid UUID format" });
  }

  let claims: JwtClaims;
  let secret = VALID_SECRET;

  switch (tokenKind) {
    case "Valid":
      claims = {
        sub: userId,
        iat: now(),
        nbf: now(),
        exp: now() + 3600,
        token_type: "access",
        is_verified: isVerified,
      };
      break;

    case "Expired":
      claims = {
        sub: userId,
        iat: now() - 7200,
        nbf: now() - 7200,
        exp: now() - 60,
        token_type: "access",
        is_verified: isVerified,
      };
      break;

    case "NotYetValid":
      claims = {
        sub: userId,
        iat: now(),
        nbf: now() + 300, // > leeway
        exp: now() + 3600,
        token_type: "access",
        is_verified: isVerified,
      };
      break;

    case "InvalidSignature":
      claims = {
        sub: userId,
        iat: now(),
        nbf: now(),
        exp: now() + 3600,
        token_type: "access",
        is_verified: isVerified,
      };
      secret = INVALID_SECRET;
      break;

    case "Malformed":
      return res.json({
        token: `malformed.${randomUUID()}.token`,
      });

    default:
      return res.status(400).json({
        error: `Unknown token_kind: ${tokenKind}`,
      });
  }

  const token = jwt.sign(claims, secret, {
    algorithm: "HS256" as Algorithm,
    noTimestamp: true, // prevent jsonwebtoken from injecting iat
  });

  res.json({ token });
});

/**
 * Generate random credentials for testing
 *
 * GET /account/random
 */
app.get("/account/random", (_req, res) => {
  const randomString = (length: number): string =>
    Array.from({ length }, () =>
      Math.floor(Math.random() * 36).toString(36),
    ).join("");

  const email = `user_${randomString(10)}@example.test`;

  // user_name: 3–50 chars, alphanumeric + underscore
  const user_name = `user_${randomString(6)}`; // length = 11

  // password: minimum length 12
  const password = randomString(16);

  res.json({
    email,
    user_name,
    password,
  });
});

/**
 * Atomically delete resumes + user for test cleanup
 *
 * DELETE /cleanup/all/:user_id
 */
app.delete("/cleanup/all/:user_id", async (req, res) => {
  const { user_id } = req.params;

  if (!/^[0-9a-fA-F-]{36}$/.test(user_id)) {
    return res.status(400).json({ error: "Invalid UUID format" });
  }

  const client = await pool.connect();

  try {
    await client.query("BEGIN");

    const resumesResult = await client.query(
      "DELETE FROM resumes WHERE user_id = $1",
      [user_id],
    );

    const userResult = await client.query("DELETE FROM users WHERE id = $1", [
      user_id,
    ]);

    if (userResult.rowCount === 0) {
      throw new Error("User not found");
    }

    await client.query("COMMIT");

    res.json({
      deleted_resumes: resumesResult.rowCount,
      deleted_users: userResult.rowCount,
    });
  } catch (err) {
    await client.query("ROLLBACK");
    console.error("Transactional cleanup failed", err);

    res.status(500).json({
      error: "Transactional cleanup failed",
      reason: err instanceof Error ? err.message : "unknown",
    });
  } finally {
    client.release();
  }
});

app.listen(4001, () => {
  console.log("🧪 JWT test helper (Bun) running on http://localhost:4001");
});
