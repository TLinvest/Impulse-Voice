import QtQuick
import Quickshell
import Quickshell.Io
pragma Singleton

Singleton {
    id: root

    readonly property string socketPath: `${Quickshell.env("XDG_RUNTIME_DIR")}/impulse-voice.sock`
    readonly property bool connected: voiceSocket.connected
    property string state: "idle"
    property string errorMessage: ""
    property int nextRequestId: 1

    signal transcriptReady(string text)

    function send(command) {
        if (!voiceSocket.connected) {
            root.errorMessage = "Le daemon Impulse Voice est hors ligne.";
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
            root.errorMessage = `Réponse invalide: ${error}`;
            errorTimeout.restart();
            return ;
        }
        if (payload.state)
            root.state = payload.state;

        if (payload.type === "transcript") {
            root.transcriptReady(payload.text ?? "");
        } else if (payload.type === "error") {
            root.errorMessage = payload.message ?? "Erreur inconnue";
            errorTimeout.restart();
        }
    }

    Socket {
        id: voiceSocket

        path: root.socketPath
        connected: true
        onError: (error) => {
            root.state = "idle";
            reconnectTimer.restart();
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
        id: errorTimeout

        interval: 3500
        onTriggered: root.errorMessage = ""
    }

}
