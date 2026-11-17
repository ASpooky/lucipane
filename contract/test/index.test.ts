import { describe, expect, it } from "vitest";
import { CONTRACT_VERSION } from "../src/index.ts";

describe("contract", () => {
  it("exposes a contract version", () => {
    expect(CONTRACT_VERSION).toBe(0);
  });
});
