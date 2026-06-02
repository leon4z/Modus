import { cleanup, render, screen } from "@testing-library/svelte";
import { afterEach, describe, expect, it } from "vitest";

import PrimaryState from "$lib/shared/components/PrimaryState.svelte";

describe("PrimaryState", () => {
  afterEach(() => {
    cleanup();
  });

  it("preserves accessible status text and stable state tone", () => {
    render(PrimaryState, {
      props: {
        message: "No project Rules found in this project.",
        detail: "Visual fixture detail",
        tone: "error",
      },
    });

    const status = screen.getByRole("status");
    expect(status).toHaveTextContent("No project Rules found in this project.");
    expect(status).toHaveTextContent("Visual fixture detail");
    expect(status).toHaveAttribute("data-tone", "error");
  });
});
