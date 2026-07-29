import QtQuick
import Quickshell
import Quickshell.Io
pragma Singleton

Singleton {
    id: root

    readonly property string socketPath: `${Quickshell.env("XDG_RUNTIME_DIR")}/impulse-voice.sock`
    readonly property bool connected: voiceSocket.connected
    property string state: "idle"
    property string visualState: "idle"
    property real audioLevel: 0
    property bool modelReady: false
    property string errorMessage: ""
    property string lastTranscript: ""
    property int nextRequestId: 1

    signal transcriptReady(string text)

    function send(command) {
        if (!voiceSocket.connected) {
            root.errorMessage = "The Impulse Voice daemon is offline.";
            errorTimeout.restart();
            return ;
        }
        voiceSocket.write(`${JSON.stringify({
            id: root.nextRequestId++,
            command: command
        })}\n`);
        voiceSocket.flush();
    }

    function start() {
        root.send("start");
    }

    function stop() {
        root.send("stop");
    }

    function toggle() {
        root.send("toggle");
    }

    function cancel() {
        root.send("cancel");
    }

    function handleMessage(line) {
        let payload;
        try {
            payload = JSON.parse(line);
        } catch (error) {
            root.errorMessage = `Invalid response: ${error}`;
            errorTimeout.restart();
            return ;
        }
        if (payload.state)
            root.state = payload.state;

        if (payload.model_ready !== undefined)
            root.modelReady = payload.model_ready;

        if (payload.type === "transcript") {
            root.lastTranscript = payload.text ?? "";
            root.audioLevel = 0;
            root.transcriptReady(root.lastTranscript);
            if (payload.paste_error) {
                root.visualState = "error";
                root.errorMessage = payload.paste_error;
                errorTimeout.restart();
            } else {
                root.visualState = "success";
                successTimeout.restart();
            }
        } else if (payload.type === "error") {
            root.audioLevel = 0;
            root.visualState = "error";
            root.errorMessage = payload.message ?? "Unknown error";
            errorTimeout.restart();
        } else if (payload.type === "meter") {
            if (root.state === "listening")
                root.audioLevel = Math.max(0, Math.min(1, payload.level ?? 0));
        } else if (payload.type === "state") {
            root.visualState = root.state;
            if (root.state !== "listening")
                root.audioLevel = 0;
        }
    }

    Socket {
        id: voiceSocket

        path: root.socketPath
        connected: true
        onError: (error) => {
            root.state = "idle";
            root.visualState = "idle";
            root.audioLevel = 0;
            reconnectTimer.restart();
        }
        onConnectedChanged: {
            if (connected)
                root.send("status");

        }

        parser: SplitParser {
            onRead: (data) => {
                return root.handleMessage(data);
            }
        }

    }

    Timer {
        id: reconnectTimer

        interval: 1500
        onTriggered: voiceSocket.connected = true
    }

    Timer {
        id: meterTimer

        interval: 40
        repeat: true
        running: root.connected && root.state === "listening"
        onTriggered: root.send("meter")
    }

    Timer {
        id: successTimeout

        interval: 520
        onTriggered: root.visualState = "idle"
    }

    Timer {
        id: errorTimeout

        interval: 3500
        onTriggered: {
            root.errorMessage = "";
            root.visualState = "idle";
        }
    }

}
