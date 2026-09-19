# Debug Session: titlebar-drag-stuck

- **Status**: [OPEN]
- **Issue**: Dragging the custom title bar can leave Windows in a persistent move state until the application is restarted.
- **Debug Server**: http://127.0.0.1:7777/event
- **Log File**: `.dbg/trae-debug-log-titlebar-drag-stuck.ndjson`

## Reproduction Steps

1. Start the Windows application.
2. Drag the window using the center area of the custom title bar.
3. Release the mouse button and move the cursor over the application.
4. Observe whether Windows remains in move mode and blocks clicks.

## Hypotheses & Verification

| ID  | Hypothesis                                                                                                        | Likelihood | Effort | Evidence |
| --- | ----------------------------------------------------------------------------------------------------------------- | ---------- | ------ | -------- |
| A   | `WM_NCLBUTTONDOWN` starts a modal move loop that does not receive the corresponding mouse-up event.               | High       | Low    | Confirmed |
| B   | `ReleaseCapture` releases Slint's capture before Windows can associate the mouse-up with the native caption drag. | High       | Low    | Confirmed |
| C   | The title-bar callback fires multiple times for one pointer gesture.                                              | Medium     | Low    | Rejected |
| D   | A fallback HWND is selected instead of the primary application window.                                            | Low        | Low    | Rejected |

## Log Evidence

- Pre-fix: the native drag sequence logged `down`, `ReleaseCapture`, and `SendMessageW` return, but no matching Slint `up` event.
- Post-fix: pending user verification. The drag path no longer releases Slint capture or sends `WM_NCLBUTTONDOWN`.

## Verification Conclusion
The native caption drag handoff is the root cause. The replacement keeps pointer ownership in Slint and moves the HWND directly while the pointer remains down.
