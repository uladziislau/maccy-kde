import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Window {
    id: window

    visible: false
    flags: Qt.FramelessWindowHint | Qt.WindowStaysOnTopHint
    color: "transparent"

    SystemPalette { id: pal; colorGroup: SystemPalette.Active }

    property string searchQuery: ""

    Timer {
        id: focusCheck
        interval: 300
        repeat: true
        running: visible
        onTriggered: {
            if (!window.active && window.visible) {
                window.hide()
            }
        }
    }

    onVisibleChanged: {
        if (visible) searchField.forceActiveFocus()
    }

    // Only Escape needs Window-level Shortcut (closes even if search unfocused)
    Shortcut { sequence: "Escape"; onActivated: window.hide() }

    Rectangle {
        anchors.fill: parent
        radius: 10
        color: pal.window
        border { color: pal.mid; width: 1 }

        ColumnLayout {
            anchors.fill: parent
            spacing: 0

            // ── Search ───────────────────────────────────
            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 48
                color: pal.alternateBase
                radius: 10

                TextField {
                    id: searchField
                    anchors { fill: parent; margins: 8 }
                    placeholderText: "Search clips…"
                    font.pointSize: 11
                    focus: true
                    background: Item {}

                    onTextChanged: {
                        searchQuery = text
                        backend.filter(text)
                    }

                    Keys.onPressed: (event) => {
                        // ── Central hotkey registry ──────────────────────
                        // Nav      : N/B, Tab/Shift+Tab
                        // Actions  : P (pin), Delete (remove)
                        // Paste    : Q/A/Z/W/S/X/E/D/R (pinned)
                        // Paste    : 1..9 (recent)

                        var ctrl = event.modifiers & Qt.ControlModifier

                        if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
                            pasteCurrent(); event.accepted = true
                        } else if (ctrl && event.key === Qt.Key_N) {
                            itemList.incrementCurrentIndex(); event.accepted = true
                        } else if (ctrl && event.key === Qt.Key_B) {
                            itemList.decrementCurrentIndex(); event.accepted = true
                        } else if (ctrl && event.key === Qt.Key_Tab) {
                            cycleCategory(event.modifiers & Qt.ShiftModifier ? -1 : 1)
                            event.accepted = true
                        } else if (ctrl && event.key === Qt.Key_P) {
                            pinCurrent(); event.accepted = true
                        } else if (ctrl && event.key === Qt.Key_Delete) {
                            deleteCurrent(); event.accepted = true
                        } else if (ctrl && event.key >= Qt.Key_0 && event.key <= Qt.Key_9) {
                            quickPaste(String.fromCharCode(event.key))
                            event.accepted = true
                        } else if (ctrl && event.key >= Qt.Key_A && event.key <= Qt.Key_Z) {
                            var letter = String.fromCharCode(event.key)
                            if (letter !== "C") quickPaste(letter)
                            event.accepted = true  // prevent DefaultChars in search
                        }
                    }
                }
            }

            // ── Category filter ─────────────────────────
            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: catRow.implicitHeight + 8
                color: pal.window

                Row {
                    id: catRow
                    anchors { left: parent.left; right: parent.right; margins: 8 }
                    spacing: 4

                    Repeater {
                        model: backend.categories
                        delegate: Button {
                            text: modelData
                            flat: true
                            highlighted: backend.activeCategory === modelData
                            font.pointSize: 9
                            onClicked: backend.filterByCategory(modelData)
                        }
                    }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 1
                color: pal.mid
            }

            // ── List ─────────────────────────────────────
            Item {
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true

                ListView {
                    id: itemList
                    anchors.fill: parent
                    boundsBehavior: Flickable.StopAtBounds
                    model: backend.itemsModel
                    currentIndex: -1

                    section.property: "section"
                    section.labelPositioning: ViewSection.InlineLabels
                    section.delegate: Rectangle {
                        width: parent.width
                        height: 26
                        color: pal.window
                        Label {
                            anchors { left: parent.left; leftMargin: 12; verticalCenter: parent.verticalCenter }
                            text: section
                            font.pointSize: 9
                            font.bold: true
                            color: pal.mid
                        }
                        Rectangle {
                            anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
                            height: 1
                            color: pal.mid
                            visible: section === "Pinned"
                        }
                    }

                    delegate: ItemDelegate {
                        id: delegate
                        width: ListView.view.width
                        height: 48
                        highlighted: ListView.isCurrentItem
                        palette.text: pal.text
                        palette.highlight: pal.highlight
                        palette.highlightedText: pal.highlightedText

                        RowLayout {
                            anchors { fill: parent; leftMargin: 4; rightMargin: 8 }
                            spacing: 4

                            Button {
                                id: pinBtn
                                text: modelData.pinned ? "★" : "☆"
                                flat: true
                                font.pointSize: 12
                                Layout.preferredWidth: 28
                                Layout.preferredHeight: 28
                                onClicked: backend.togglePin(modelData.text)
                            }

                            Label {
                                id: textLabel
                                Layout.fillWidth: true
                                Layout.fillHeight: true
                                text: modelData.text
                                elide: Text.ElideRight
                                maximumLineCount: 1
                                verticalAlignment: Text.AlignVCenter
                                font.pointSize: 10
                                color: ListView.isCurrentItem ? pal.highlightedText : pal.text
                            }

                            Label {
                                text: modelData.category
                                font.pointSize: 8
                                color: pal.buttonText
                                padding: 4
                                background: Rectangle {
                                    radius: 4
                                    color: pal.button
                                }
                            }

                            Rectangle {
                                visible: modelData.shortcut !== ""
                                width: 24
                                height: 20
                                radius: 4
                                color: pal.dark
                                Label {
                                    anchors.centerIn: parent
                                    text: modelData.shortcut || ""
                                    font.pointSize: 9
                                    font.bold: modelData.pinned
                                    color: pal.light
                                }
                            }
                        }

                        ToolTip {
                            visible: delegate.hovered
                            text: modelData.text
                            delay: 600
                            font.pointSize: 9
                        }

                        onClicked: {
                            itemList.currentIndex = index
                            pasteCurrent()
                        }
                    }

                    ScrollBar.vertical: ScrollBar {
                        active: true
                    }
                }

                // ── Empty state ─────────────────────────
                Label {
                    anchors.centerIn: parent
                    visible: itemList.count === 0
                    text: searchQuery ? "No matching clips" : "Clipboard history is empty"
                    font.pointSize: 12
                    color: pal.mid
                }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 1
                color: pal.mid
            }

            // ── Status bar ──────────────────────────────
            RowLayout {
                Layout.fillWidth: true
                Layout.preferredHeight: 32
                Layout.leftMargin: 8
                Layout.rightMargin: 8

                Button {
                    text: "Clear Pins"
                    flat: true
                    font.pointSize: 9
                    onClicked: backend.clearAllPins()
                }

                Item { Layout.fillWidth: true }

                Label {
                    text: "Ctrl+P · N/B · Tab · Del · Ctrl+letter #9"
                    font.pointSize: 8
                    color: pal.mid
                }
            }
        }
    }

    function cycleCategory(dir) {
        var cats = backend.categories
        var cur = backend.activeCategory
        var idx = cats.indexOf(cur)
        if (idx < 0) idx = 0
        idx = (idx + dir + cats.length) % cats.length
        backend.filterByCategory(cats[idx])
    }

    function quickPaste(key) {
        var items = backend.itemsModel
        for (var i = 0; i < items.length; i++) {
            if (items[i].shortcut === key) {
                window.hide()
                backend.paste(items[i].text)
                return
            }
        }
    }

    function pinCurrent() {
        if (itemList.currentIndex >= 0 && itemList.currentIndex < itemList.count) {
            var data = backend.itemsModel[itemList.currentIndex]
            if (data && data.text) {
                backend.togglePin(data.text)
            }
        }
    }

    function deleteCurrent() {
        if (itemList.currentIndex >= 0 && itemList.currentIndex < itemList.count) {
            var data = backend.itemsModel[itemList.currentIndex]
            if (data && data.text) {
                backend.deleteItem(data.text)
            }
        }
    }

    function pasteCurrent() {
        if (itemList.currentIndex >= 0 && itemList.currentIndex < itemList.count) {
            var data = backend.itemsModel[itemList.currentIndex]
            if (data && data.text) {
                window.hide()
                backend.paste(data.text)
            }
        }
    }
}
