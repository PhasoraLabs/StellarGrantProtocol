import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { useState, type ComponentProps } from "react";
import { Controller, useForm } from "react-hook-form";
import {
  AddressInput,
  addressToColor,
} from "@/components/ui/AddressInput";

const VALID_ADDRESS = `G${"A".repeat(55)}`;
const OTHER_VALID_ADDRESS = `G${"B".repeat(55)}`;

vi.mock("@/lib/store", () => ({
  useWalletStore: vi.fn((selector: (s: { address: string | null }) => unknown) =>
    selector({ address: `G${"W".repeat(55)}` }),
  ),
}));

describe("addressToColor", () => {
  it("returns deterministic colours per address", () => {
    expect(addressToColor(VALID_ADDRESS)).toBe(addressToColor(VALID_ADDRESS));
    expect(addressToColor(VALID_ADDRESS)).not.toBe(addressToColor(OTHER_VALID_ADDRESS));
  });
});

describe("AddressInput", () => {
  function ControlledInput(
    props: Partial<ComponentProps<typeof AddressInput>> & {
      initial?: string;
    },
  ) {
    const [value, setValue] = useState(props.initial ?? "");
    return (
      <AddressInput
        value={value}
        onChange={setValue}
        label="Recipient"
        {...props}
      />
    );
  }

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("shows green check immediately when pasting a valid address", () => {
    render(<ControlledInput />);
    const input = screen.getByLabelText("Recipient");

    fireEvent.paste(input, {
      clipboardData: { getData: () => VALID_ADDRESS },
    });
    fireEvent.change(input, { target: { value: VALID_ADDRESS } });

    expect(input.parentElement?.querySelector(".text-success")).toBeTruthy();
  });

  it("shows red X and error only after blur when pasting invalid text", () => {
    render(<ControlledInput />);
    const input = screen.getByLabelText("Recipient");

    fireEvent.paste(input, {
      clipboardData: { getData: () => "not-a-stellar-address" },
    });
    fireEvent.change(input, { target: { value: "not-a-stellar-address" } });

    expect(screen.queryByText("Invalid Stellar address")).toBeNull();

    fireEvent.blur(input);

    expect(screen.getByText("Invalid Stellar address")).toBeTruthy();
    expect(input.parentElement?.querySelector(".text-danger")).toBeTruthy();
  });

  it('fills the field when "Use my address" is clicked', async () => {
    const onValidAddress = vi.fn();
    render(
      <ControlledInput showUseMyAddress onValidAddress={onValidAddress} />,
    );

    fireEvent.click(screen.getByRole("button", { name: /use my address/i }));

    const input = screen.getByLabelText("Recipient") as HTMLInputElement;
    await waitFor(() => {
      expect(input.value).toMatch(/^G[A-Z2-7]{55}$/);
      expect(onValidAddress).toHaveBeenCalledWith(input.value);
    });
  });

  it("renders identicon with address prefix when valid", () => {
    render(<ControlledInput showAvatar initial={VALID_ADDRESS} />);
    const input = screen.getByLabelText("Recipient");
    fireEvent.blur(input);

    expect(screen.getByText(VALID_ADDRESS.slice(0, 2))).toBeTruthy();
  });

  it("works inside a React Hook Form Controller", async () => {
    function FormWrapper() {
      const { control } = useForm({ defaultValues: { address: "" } });
      return (
        <Controller
          name="address"
          control={control}
          render={({ field }) => (
            <AddressInput
              {...field}
              label="Address"
              onChange={field.onChange}
            />
          )}
        />
      );
    }

    render(<FormWrapper />);
    const input = screen.getByLabelText("Address");

    fireEvent.change(input, { target: { value: VALID_ADDRESS } });
    fireEvent.blur(input);

    await waitFor(() => {
      expect((input as HTMLInputElement).value).toBe(VALID_ADDRESS);
    });
  });

  it("does not show validation while typing before blur", () => {
    render(<ControlledInput />);
    const input = screen.getByLabelText("Recipient");

    fireEvent.change(input, { target: { value: "GABC" } });

    expect(input.parentElement?.querySelector(".text-success")).toBeNull();
    expect(input.parentElement?.querySelector(".text-danger")).toBeNull();
  });

  it("validates on blur for manually entered valid addresses", () => {
    const onValidAddress = vi.fn();
    render(<ControlledInput onValidAddress={onValidAddress} />);
    const input = screen.getByLabelText("Recipient");

    fireEvent.change(input, { target: { value: VALID_ADDRESS } });
    fireEvent.blur(input);

    expect(input.parentElement?.querySelector(".text-success")).toBeTruthy();
    expect(onValidAddress).toHaveBeenCalledWith(VALID_ADDRESS);
  });
});
