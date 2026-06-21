import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Window {
    id: window

    visible: false
    width: 420
    height: 520
    flags: Qt.FramelessWindowHint | Qt.WindowStaysOnTopHint
    color: "transparent"

    SystemPalette { id: pal; colorGroup: SystemPalette.Active }

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

    Shortcut { sequence: "Escape"; onActivated: window.hide() }
    Shortcut { sequence: "Ctrl+P"; onActivated: pinCurrent() }

    // Quick-paste: pinned items (Ctrl + letter)
    Shortcut { sequence: "Ctrl+Q"; onActivated: quickPaste("Q") }
    Shortcut { sequence: "Ctrl+A"; onActivated: quickPaste("A") }
    Shortcut { sequence: "Ctrl+Z"; onActivated: quickPaste("Z") }
    Shortcut { sequence: "Ctrl+W"; onActivated: quickPaste("W") }
    Shortcut { sequence: "Ctrl+S"; onActivated: quickPaste("S") }
    Shortcut { sequence: "Ctrl+X"; onActivated: quickPaste("X") }
    Shortcut { sequence: "Ctrl+E"; onActivated: quickPaste("E") }
    Shortcut { sequence: "Ctrl+D"; onActivated: quickPaste("D") }
    Shortcut { sequence: "Ctrl+C"; onActivated: quickPaste("C") }

    // Quick-paste: recent items (Ctrl + digit — Meta+digit conflicts with KDE)
    Shortcut { sequence: "Ctrl+1"; onActivated: quickPaste("1") }
    Shortcut { sequence: "Ctrl+2"; onActivated: quickPaste("2") }
    Shortcut { sequence: "Ctrl+3"; onActivated: quickPaste("3") }
    Shortcut { sequence: "Ctrl+4"; onActivated: quickPaste("4") }
    Shortcut { sequence: "Ctrl+5"; onActivated: quickPaste("5") }
    Shortcut { sequence: "Ctrl+6"; onActivated: quickPaste("6") }
    Shortcut { sequence: "Ctrl+7"; onActivated: quickPaste("7") }
    Shortcut { sequence: "Ctrl+8"; onActivated: quickPaste("8") }
    Shortcut { sequence: "Ctrl+9"; onActivated: quickPaste("9") }

    Rectangle {
        anchors.fill: parent
        radius: 8
        color: pal.window
        border { color: pal.mid; width: 1 }

        ColumnLayout {
            anchors.fill: parent
            spacing: 0

            Rectangle {
                id: searchBox
                Layout.fillWidth: true
                Layout.preferredHeight: 44
                color: pal.alternateBase
                radius: 8

                TextField {
                    id: searchField
                    anchors { fill: parent; margins: 8 }
                    placeholderText: "Search..."
                    font.pointSize: 11
                    focus: true
                    background: Item {}

                    onTextChanged: backend.filter(searchField.text)

                    Keys.onReturnPressed: pasteCurrent()
                    Keys.onUpPressed: itemList.decrementCurrentIndex()
                    Keys.onDownPressed: itemList.incrementCurrentIndex()
                }
            }

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

            ListView {
                id: itemList
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                boundsBehavior: Flickable.StopAtBounds
                model: backend.itemsModel

                section.property: "section"
                section.labelPositioning: ViewSection.InlineLabels
                section.delegate: Rectangle {
                    width: parent.width
                    height: 28
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
                    width: ListView.view.width
                    height: 48
                    highlighted: ListView.isCurrentItem
                    palette.text: pal.text
                    palette.highlight: pal.highlight
                    palette.highlightedText: pal.highlightedText

                    RowLayout {
                        anchors { fill: parent; leftMargin: 12; rightMargin: 8 }
                        spacing: 8

                        Label {
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            text: modelData.text
                            elide: Text.ElideRight
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

                        Label {
                            text: modelData.shortcut || ""
                            font.pointSize: 9
                            font.bold: modelData.pinned
                            color: ListView.isCurrentItem ? pal.highlightedText : pal.mid
                            Layout.preferredWidth: 24
                            horizontalAlignment: Text.AlignHCenter
                        }
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

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 1
                color: pal.mid
            }

            RowLayout {
                Layout.fillWidth: true
                Layout.preferredHeight: 36
                Layout.leftMargin: 8
                Layout.rightMargin: 8

                Button {
                    text: "Clear All Pins"
                    flat: true
                    font.pointSize: 9
                    onClicked: backend.clearUnpinned()
                }

                Item { Layout.fillWidth: true }

                Label {
                    text: "Ctrl+P pin  •  Ctrl+letter  •  Ctrl+#  •  ✨ hot-reload"
                    font.pointSize: 8
                    color: pal.mid
                }
            }
        }
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
