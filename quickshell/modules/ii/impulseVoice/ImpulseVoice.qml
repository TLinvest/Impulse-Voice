pragma ComponentBehavior: Bound

import qs.services
import qs.modules.common
import qs.modules.common.widgets
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Hyprland
import Quickshell.Io
import Quickshell.Wayland

Scope {
    id: root

    readonly property bool shouldShow: ImpulseVoiceService.state !== "idle"
        || ImpulseVoiceService.errorMessage !== ""
    property var focusedScreen: Quickshell.screens.find(
        screen => screen.name === Hyprland.focusedMonitor?.name
    )

    function statusText() {
        if (ImpulseVoiceService.errorMessage !== "")
            return ImpulseVoiceService.errorMessage;
        if (ImpulseVoiceService.state === "listening")
            return "Écoute…";
        if (ImpulseVoiceService.state === "processing")
            return "Transcription locale…";
        return "Impulse Voice";
    }

    Loader {
        id: capsuleLoader
        active: root.shouldShow

        sourceComponent: PanelWindow {
            id: capsuleWindow
            screen: root.focusedScreen
            color: "transparent"
            focusable: false
            exclusiveZone: 0
            exclusionMode: ExclusionMode.Ignore

            WlrLayershell.namespace: "quickshell:impulseVoice"
            WlrLayershell.layer: WlrLayer.Overlay

            anchors.bottom: true
            margins.bottom: Appearance.sizes.barHeight + 28
            implicitWidth: capsule.implicitWidth
            implicitHeight: capsule.implicitHeight

            Rectangle {
                id: capsule
                implicitWidth: Math.max(260, content.implicitWidth + 40)
                implicitHeight: 58
                radius: height / 2
                color: Appearance.colors.colLayer1
                border.width: 1
                border.color: Appearance.colors.colOutlineVariant

                RowLayout {
                    id: content
                    anchors.centerIn: parent
                    spacing: 12

                    Rectangle {
                        Layout.preferredWidth: 14
                        Layout.preferredHeight: 14
                        radius: 7
                        color: ImpulseVoiceService.errorMessage !== ""
                            ? Appearance.m3colors.m3error
                            : Appearance.colors.colPrimary

                        SequentialAnimation on opacity {
                            running: ImpulseVoiceService.state === "listening"
                            loops: Animation.Infinite
                            NumberAnimation { to: 0.3; duration: 450 }
                            NumberAnimation { to: 1.0; duration: 450 }
                        }
                    }

                    StyledText {
                        text: root.statusText()
                        color: Appearance.colors.colOnLayer1
                        font.pixelSize: Appearance.font.pixelSize.normal
                        font.weight: Font.Medium
                    }
                }
            }
        }
    }

    IpcHandler {
        target: "impulseVoice"

        function start(): void {
            ImpulseVoiceService.start();
        }

        function stop(): void {
            ImpulseVoiceService.stop();
        }

        function toggle(): void {
            ImpulseVoiceService.toggle();
        }

        function cancel(): void {
            ImpulseVoiceService.cancel();
        }
    }

    GlobalShortcut {
        name: "impulseVoiceToggle"
        description: "Démarre ou arrête Impulse Voice"
        onPressed: ImpulseVoiceService.toggle()
    }

    GlobalShortcut {
        name: "impulseVoiceCancel"
        description: "Annule la transcription Impulse Voice"
        onPressed: ImpulseVoiceService.cancel()
    }
}
