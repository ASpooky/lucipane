import { CONTRACT_VERSION } from "@lucipane/contract";
import { describe, expect, it } from "vitest";

describe("daemon", () => {
  it("resolves the workspace contract package", () => {
    expect(CONTRACT_VERSION).toBe(0);
  });
});
