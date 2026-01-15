#!/bin/bash
# Helper to launch flt in a new iTerm2 window and proxy stdout.

FLT_BIN="$1"
ASSETS_DIR="$2"
ICU_DATA="$3"

# Used to communicate between the flt process and the Flutter Tool.
LOG_FILE="/tmp/flt_log_$$"
RUN_FLAG="/tmp/flt_run_$$"

# Create log file and run flag
touch "$LOG_FILE"
touch "$RUN_FLAG"

# Construct the command to run in iTerm2.
CMD="$FLT_BIN --assets-dir $ASSETS_DIR --icu-data-path $ICU_DATA --log-file $LOG_FILE"
WRAPPER_CMD="$CMD; rm $RUN_FLAG; exit"

echo "Launching flt in new iTerm2 window..."

# Launch iTerm2 and get the Window ID
WINDOW_ID=$(osascript -e "tell application \"iTerm\"
    set newWindow to (create window with default profile)
    tell current session of newWindow
        write text \"$WRAPPER_CMD\"
    end tell
    get id of newWindow
end tell")

echo "Launched window: $WINDOW_ID"

# Function to clean up
cleanup() {
    # Kill background tail
    if [ -n "$TAIL_PID" ]; then kill $TAIL_PID 2>/dev/null; fi

    # Remove temp files
    rm "$LOG_FILE" "$RUN_FLAG" 2>/dev/null

    # Close the iTerm2 window
    if [ -n "$WINDOW_ID" ]; then
        osascript -e "tell application \"iTerm\"
            if exists window id $WINDOW_ID then
                close window id $WINDOW_ID
            end if
        end tell"
    fi
}

# Trap EXIT for cleanup.
trap cleanup EXIT
# Trap signals to force exit, which triggers EXIT trap.
trap "exit 1" SIGINT SIGTERM

# Tail the log file to stdout
tail -f "$LOG_FILE" &
TAIL_PID=$!

# Wait for flt to exit (detected by removal of RUN_FLAG)
while [ -f "$RUN_FLAG" ]; do
    sleep 0.5
done

