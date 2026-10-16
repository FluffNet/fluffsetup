import QtQuick
import QtPositioning

Item {
    id: root
    objectName: "kde-timezone-map"
    property string zone: ""
    property string language: "en"
    property var center: null
    property real preferredZoom: 4
    property var mapText: (context, source) => source
    readonly property var mapItem: selector.map
    readonly property var mapView: selector
    readonly property var surface: selector
    property var areaBounds: null
    property bool polarCoverageLimited: false
    property string fittedZone: ""
    property real viewportWidth: width
    property real viewportHeight: height
    property int fitMargin: 32
    property int centerAttempts: 0
    readonly property bool fitting: centerTimer.running || settleTimer.running
    property var targetCenter: null
    property real targetZoom: 0
    readonly property string highlightedZone: selector.selectedTimeZone
    signal zoneSelected(string id)

    TimezoneMapSurface {
        id: selector
        anchors.centerIn: parent
        width: root.viewportWidth
        height: root.viewportHeight
        selectedTimeZone: root.zone
        initialCenter: root.center
        initialZoom: root.zone.length ? 4.9 : root.preferredZoom
        opacity: root.fittedZone === root.zone && !root.fitting ? 1 : 0
        enabled: !root.fitting
        mapText: (context, source) => {
            const language = root.language
            return root.mapText(context, source)
        }
        onZoneSelected: id => root.zoneSelected(id)
    }
    // Fit the complete geometry without waiting for visual delegates to render.
    function selectedCoordinates() {
        const coordinates = []
        for (const row of selector.polygons)
            if (row.tzid === selector.selectedTimeZone)
                for (const coordinate of row.coordinates) coordinates.push(coordinate)
        return coordinates
    }
    function selectedBounds() {
        const coordinates = selectedCoordinates()
        return coordinates.length ? QtPositioning.rectangle(coordinates) : null
    }
    function fitViewport(bounds) {
        if (width <= 0 || height <= 0) return false
        const top = QtPositioning.coordToMercator(bounds.topLeft)
        const bottom = QtPositioning.coordToMercator(bounds.bottomRight)
        const spanX = Math.max(bounds.width / 360, 0.000001)
        const spanY = Math.max(Math.abs(bottom.y - top.y), 0.000001)
        const gap = Math.max(0, 1 - Math.max(spanX, spanY))
        // Qt will not zoom below the level that fills the larger viewport axis.
        // Letterbox only when that minimum would otherwise crop a large area
        // in a very wide/portrait map. Keep as much native map space as possible.
        // Near a Mercator pole the camera cannot center the outline exactly.
        // Limit padding to the actual map coverage above/below the polygon,
        // so Qt's camera clamp still leaves every border point visible.
        const poleGap = Math.max(0, Math.min(top.y, 1 - bottom.y))
        const maximumMargin = Math.min(width * gap / (2 * (gap + spanX)), height * gap / (2 * (gap + spanY)),
            width * poleGap / (spanX + 2 * poleGap), height * poleGap / (spanY + 2 * poleGap))
        fitMargin = Math.max(0, Math.floor(Math.min(32, maximumMargin * 0.95)))
        const worldPixels = Math.min((width - 2 * fitMargin) / spanX, (height - 2 * fitMargin) / spanY)
        const nextWidth = Math.min(width, worldPixels)
        const nextHeight = Math.min(height, worldPixels)
        if (Math.abs(viewportWidth - nextWidth) > 0.5 || Math.abs(viewportHeight - nextHeight) > 0.5) {
            viewportWidth = nextWidth
            viewportHeight = nextHeight
            return false // Let the native viewport/minimum zoom update first.
        }
        mapItem.fitViewportToGeoShape(bounds, fitMargin)
        // The native fit sets center before zoom, so a cold portrait map can
        // temporarily clamp that center at its previous minimum zoom. Derive
        // the same Mercator midpoint from the outline, not that clamped getter.
        root.targetCenter = QtPositioning.mercatorToCoord(Qt.point((top.x + spanX / 2) % 1, (top.y + bottom.y) / 2))
        root.targetZoom = mapItem.zoomLevel
        settleTimer.passes = 0
        settleTimer.restart()
        return true
    }
    function styleZoomButtons(item) {
        // Use the installer's existing faces on the native-map zoom controls.
        if (item.clicked !== undefined && item.icon !== undefined
                && ["zoom-in-map-symbolic", "zoom-out-map-symbolic"].includes(item.icon.name)
                && !item.objectName.startsWith("fluff-timezone-zoom-")) {
            const zoomIn = item.icon.name === "zoom-in-map-symbolic"
            item.objectName = zoomIn ? "fluff-timezone-zoom-in" : "fluff-timezone-zoom-out"
            item.contentItem = zoomLabel.createObject(item, {control: item, glyph: zoomIn ? "+" : "-"})
            item.background = zoomBackground.createObject(item, {control: item})
            // Keep an input target larger than the visible button face.
            item.implicitWidth = 48
            item.implicitHeight = 44
            item.hoverEnabled = true
            item.leftPadding = 0
            item.rightPadding = 0
            item.topPadding = 0
            item.bottomPadding = 0
        }
        for (let child of item.children) styleZoomButtons(child)
    }
    Component {
        id: zoomLabel
        Text {
            required property var control
            required property string glyph
            objectName: "timezone-zoom-label"
            text: glyph
            color: control.enabled ? "#111111" : "#85888e"
            font.pixelSize: 24
            font.weight: Font.DemiBold
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
        }
    }
    Component {
        id: zoomBackground
        Item {
            required property var control
            Rectangle {
                anchors.centerIn: parent
                width: 44 + (control.visualFocus ? 6 : 0)
                height: 36 + (control.visualFocus ? 6 : 0)
                radius: 9
                color: "transparent"
                border.width: control.visualFocus ? 3 : 0
                border.color: "#111111"
            }
            Rectangle {
                objectName: "timezone-zoom-fill"
                anchors.centerIn: parent
                width: 44
                height: 36
                radius: 6
                color: !control.enabled ? "#f4f4f5" : control.down ? "#e2e4e7" : control.hovered ? "#eceef1" : "white"
                border.width: 1
                border.color: !control.enabled ? "#d0d2d6" : "#b9bcc2"
            }
        }
    }
    function requestCenter() {
        centerAttempts = 0
        fittedZone = ""
        areaBounds = null
        polarCoverageLimited = false
        viewportWidth = width
        viewportHeight = height
        settleTimer.stop()
        centerTimer.restart()
    }
    Timer {
        id: settleTimer
        property int passes: 0
        interval: 16
        repeat: true
        onTriggered: {
            if (passes < 2 && root.targetCenter && root.mapItem.mapReady) {
                root.mapItem.zoomLevel = root.targetZoom
                root.mapItem.center = root.targetCenter
            }
            // Qt Location also commits camera changes during scene-graph
            // polish. Do not expose input until those initial commits settle.
            if (++passes >= 8) stop()
        }
    }
    Connections {
        target: root.mapItem
        ignoreUnknownSignals: true
        function onMapReadyChanged() { if (root.mapItem.mapReady) root.requestCenter() }
    }
    Connections {
        target: selector
        function onPolygonsChanged() { root.requestCenter() }
    }
    Timer {
        id: centerTimer
        interval: 16
        repeat: true
        onTriggered: {
            if (!root.mapItem || !root.mapItem.mapReady) {
                if (++root.centerAttempts >= 180) stop()
                return
            }
            root.styleZoomButtons(selector)
            const bounds = root.selectedBounds()
            // The offline Web Mercator map cannot display the far-polar parts
            // of these outlines. Preserve the existing point/zoom fallback for
            // those cases; do not shrink them to a broken microscopic viewport.
            root.polarCoverageLimited = bounds !== null && (bounds.topLeft.latitude > 85.05112878 || bounds.bottomRight.latitude < -85.05112878)
            if (bounds && !root.polarCoverageLimited) {
                if (!root.fitViewport(bounds)) return
                root.areaBounds = bounds
                root.fittedZone = selector.selectedTimeZone
                stop()
                return
            }
            // Wait for geometry that KDE says is available. Unknown/unmapped
            // zones keep the existing coordinate fallback rather than faking an area.
            if (!bounds && selector.availableMapTimeZones.includes(selector.selectedTimeZone) && ++root.centerAttempts < 180) return
            stop()
            root.fittedZone = selector.selectedTimeZone
            if (!root.center || root.center.length !== 2) return
            root.targetCenter = QtPositioning.coordinate(root.center[0], root.center[1])
            root.targetZoom = Math.min(root.preferredZoom, root.mapItem.maximumZoomLevel)
            settleTimer.passes = 0
            settleTimer.restart()
        }
    }
    onCenterChanged: requestCenter()
    onWidthChanged: requestCenter()
    onHeightChanged: requestCenter()
    onZoneChanged: {
        requestCenter()
    }
    // Synchronize again when the map becomes ready. A root-owned Timer avoids
    // delayed callbacks into a Loader item destroyed during page transitions.
    Component.onCompleted: { styleZoomButtons(selector); requestCenter() }
}
