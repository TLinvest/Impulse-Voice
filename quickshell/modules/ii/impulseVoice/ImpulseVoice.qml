pragma ComponentBehavior: Bound

import qs.services
import qs.modules.common
import QtQuick
import Quickshell
import Quickshell.Hyprland
import Quickshell.Io
import Quickshell.Wayland

Scope {
    id: root

    readonly property bool shouldShow: ImpulseVoiceService.visualState !== "idle"
    property var focusedScreen: Quickshell.screens.find(
        screen => screen.name === Hyprland.focusedMonitor?.name
    )

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

            anchors.top: true
            // The bar already reserves its own layer-shell exclusive zone.
            margins.top: 8
            implicitWidth: capsule.implicitWidth
            implicitHeight: capsule.implicitHeight

            Rectangle {
                id: capsule
                property real animationPhase: 0
                property string visualState: ImpulseVoiceService.visualState

                implicitWidth: 92
                implicitHeight: 40
                radius: height / 2
                color: Appearance.colors.colLayer0
                border.width: 1
                border.color: Appearance.colors.colLayer0Border

                function barHeight(index) {
                    const state = capsule.visualState;
                    const center = (waveform.count - 1) / 2;
                    const centerWeight = 1.0 - Math.abs(index - center) / (center + 1);

                    if (state === "listening") {
                        const level = Math.max(0.035, ImpulseVoiceService.audioLevel);
                        const motion = 0.62 + 0.38 * Math.sin(
                            capsule.animationPhase * 0.18 + index * 1.37
                        );
                        const texture = 0.72 + 0.28 * Math.sin(index * 2.11 + 0.8);
                        return 4 + 21 * level * motion * texture
                            * (0.82 + centerWeight * 0.18);
                    }

                    if (state === "processing") {
                        const travel = (
                            capsule.animationPhase * 0.16
                        ) % (waveform.count + 5) - 2;
                        const distance = Math.abs(index - travel);
                        return 4 + 18 * Math.exp(-distance * distance / 2.8);
                    }

                    if (state === "success") {
                        const collapse = Math.max(
                            0,
                            1 - capsule.animationPhase / 18
                        );
                        return 4 + 13 * collapse
                            * (0.55 + centerWeight * 0.45);
                    }

                    const errorPulse = 0.5 + 0.5 * Math.sin(
                        capsule.animationPhase * 0.12 + index * 0.48
                    );
                    return 4 + 5 * errorPulse;
                }

                Timer {
                    interval: 32
                    repeat: true
                    running: capsuleWindow.visible
                    onTriggered: capsule.animationPhase++
                }

                onVisualStateChanged: capsule.animationPhase = 0

                Row {
                    id: waveform
                    anchors.centerIn: parent
                    spacing: 3
                    property int count: 11

                    Repeater {
                        model: waveform.count

                        Rectangle {
                            required property int index

                            anchors.verticalCenter: parent.verticalCenter
                            width: 3
                            height: capsule.barHeight(index)
                            radius: width / 2
                            color: capsule.visualState === "error"
                                ? Appearance.m3colors.m3error
                                : Appearance.colors.colOnLayer0

                            Behavior on height {
                                NumberAnimation {
                                    duration: 70
                                    easing.type: Easing.OutCubic
                                }
                            }

                            Behavior on color {
                                ColorAnimation { duration: 160 }
                            }
                        }
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
        name: "impulseVoiceHold"
        description: "Hold to dictate with Impulse Voice"
        onPressed: ImpulseVoiceService.start()
        onReleased: ImpulseVoiceService.stop()
    }

    GlobalShortcut {
        name: "impulseVoiceToggle"
        description: "Start or stop Impulse Voice"
        onPressed: ImpulseVoiceService.toggle()
    }

    GlobalShortcut {
        name: "impulseVoiceCancel"
        description: "Cancel the Impulse Voice recording"
        onPressed: ImpulseVoiceService.cancel()
    }
}
