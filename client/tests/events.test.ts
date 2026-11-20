import { EventParser } from "../src/events";
import { nativeToScVal } from "@stellar/stellar-sdk";

describe("EventParser", () => {
  it("parses a single successful event from XDR base64", () => {
    const mockResponse: any = {
      status: "SUCCESS",
      events: [
        {
          contractId: "CC123",
          topic: [
            nativeToScVal("GrantCreated", { type: "symbol" }).toXDR("base64"),
          ],
          value: nativeToScVal({
            event_version: 1,
            grant_id: 123n,
            owner: "GABC",
            title: "My Grant",
            total_amount: 1000n,
            tags: ["tag1"],
            timestamp: 1620000000n,
          }).toXDR("base64"),
        },
      ],
    };

    const parsed = EventParser.parseEvents(mockResponse);
    expect(parsed).toHaveLength(1);
    expect(parsed[0].name).toBe("GrantCreated");
    expect(parsed[0].data.title).toBe("My Grant");
    expect(parsed[0].data.grant_id).toBe(123n);
    expect(parsed[0].contractId).toBe("CC123");
  });

  it("handles multiple events in one transaction", () => {
    const mockResponse: any = {
      status: "SUCCESS",
      events: [
        {
          contractId: "CC123",
          topic: [nativeToScVal("Event1", { type: "symbol" }).toXDR("base64")],
          value: nativeToScVal(1).toXDR("base64"),
        },
        {
          contractId: "CC123",
          topic: [nativeToScVal("Event2", { type: "symbol" }).toXDR("base64")],
          value: nativeToScVal(2).toXDR("base64"),
        },
      ],
    };

    const parsed = EventParser.parseEvents(mockResponse);
    expect(parsed).toHaveLength(2);
    expect(parsed[0].name).toBe("Event1");
    expect(parsed[1].name).toBe("Event2");
  });

  it("returns empty array for non-successful transactions", () => {
    const mockResponse: any = {
      status: "FAILED",
      events: [],
    };
    const parsed = EventParser.parseEvents(mockResponse);
    expect(parsed).toEqual([]);
  });

  it("helper findEvent and filterEvents work correctly", () => {
      const events = [
          { name: "A", data: 1, contractId: "C1" },
          { name: "B", data: 2, contractId: "C1" },
          { name: "A", data: 3, contractId: "C1" },
      ];
      
      expect(EventParser.findEvent(events, "A")?.data).toBe(1);
      expect(EventParser.findEvent(events, "C")).toBeUndefined();
      expect(EventParser.filterEvents(events, "A")).toHaveLength(2);
  });
});
