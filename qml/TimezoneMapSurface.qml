import QtQuick
import QtQuick.Controls as Controls
import QtLocation
import QtPositioning
import QtCore
import org.kde.kirigami as Kirigami
import org.kde.plasma.workspace.timezoneselector as Workspace

Item {
    id: surface
    property string selectedTimeZone: ""
    property string hoveredTimeZone: ""
    property var initialCenter: null
    property real initialZoom: 4
    property var mapText: (context, source) => source
    readonly property alias map: view.map
    readonly property var geometryData: geometry
    // Using the installed singleton also registers KDE's offline tile resource.
    readonly property var validTimeZones: Array.from(Workspace.TimeZoneUtils.availableTimeZoneIds(), id => String(id))
    readonly property var polygons: polygonRows(geometry.model)
    property var renderBounds: null
    property var visiblePolygons: []
    readonly property var availableMapTimeZones: [...new Set(polygons.map(row => row.tzid))]
    property bool shuttingDown: false
    signal zoneSelected(string id)

    function updateViewport() {
        if (!view.map.mapReady || view.map.width <= 0 || view.map.height <= 0) {
            visiblePolygons = polygons.filter(row => row.tzid === selectedTimeZone)
            return
        }
        const topLeft = view.map.toCoordinate(Qt.point(0, 0), false)
        const bottomRight = view.map.toCoordinate(Qt.point(view.map.width, view.map.height), false)
        // Explicit corners preserve wide/dateline-crossing views. A minimal
        // rectangle around four points can select the invisible complement.
        const fullWorld = 256 * Math.pow(2, view.map.zoomLevel) <= view.map.width + 1
        renderBounds = QtPositioning.rectangle(
            fullWorld ? QtPositioning.coordinate(topLeft.latitude, -180) : topLeft,
            fullWorld ? QtPositioning.coordinate(bottomRight.latitude, 180) : bottomRight)
        visiblePolygons = polygons.filter(row => row.tzid === selectedTimeZone || intersects(row.bounds, renderBounds))
    }
    onPolygonsChanged: updateViewport()
    onSelectedTimeZoneChanged: viewportTimer.restart()
    Timer {
        id: viewportTimer
        interval: 80
        onTriggered: surface.updateViewport()
    }
    Connections {
        target: view.map
        function onCenterChanged() { viewportTimer.restart() }
        function onZoomLevelChanged() { viewportTimer.restart() }
        function onWidthChanged() { viewportTimer.restart() }
        function onHeightChanged() { viewportTimer.restart() }
        function onMapReadyChanged() { viewportTimer.restart() }
    }

    function intersects(a, b) {
        if (!a || !b || !a.isValid || !b.isValid) return false
        if (a.topLeft.latitude < b.bottomRight.latitude || a.bottomRight.latitude > b.topLeft.latitude) return false
        function segments(rect) {
            const west = rect.topLeft.longitude, east = rect.bottomRight.longitude
            return west <= east ? [[west,east]] : [[west,180],[-180,east]]
        }
        return segments(a).some(x => segments(b).some(y => x[0] <= y[1] && y[0] <= x[1]))
    }

    // Project the native GeoJsonData display model to independent polygon rows.
    // Resolve collection metadata here, never through a visual parent that Qt
    // Location can detach during model replacement or map destruction.
    function polygonRows(nodes) {
        const rows = []
        function visit(node, inheritedId) {
            if (!node) return
            const id = node.properties?.tzid || inheritedId || ""
            if (node.type === "Polygon") {
                if (typeof id === "string" && id.length && node.data) {
                    const shape = QtPositioning.shapeToPolygon(node.data)
                    const coordinates = Array.from(shape.perimeter)
                    rows.push({tzid:id,shape:shape,coordinates:coordinates,bounds:QtPositioning.rectangle(coordinates)})
                }
            } else if (["MultiPolygon", "FeatureCollection", "GeometryCollection"].includes(node.type)) {
                for (let index = 0; index < (node.data?.length || 0); ++index)
                    visit(node.data[index],id)
            }
        }
        for (let index = 0; index < (nodes?.length || 0); ++index) visit(nodes[index],"")
        return rows
    }
    function chooseTimezone(id) {
        if (!shuttingDown && validTimeZones.includes(id) && availableMapTimeZones.includes(id))
            zoneSelected(id)
    }
    GeoJsonData {
        id: geometry
        sourceUrl: StandardPaths.locate(StandardPaths.GenericDataLocation,"timezonefiles",StandardPaths.LocateDirectory)+"/timezones.json"
        onModelChanged: surface.hoveredTimeZone = ""
    }
    MapView {
        id: view
        anchors.fill: parent
        map.plugin: Plugin {
            name: "osm"
            PluginParameter { name:"osm.mapping.offline.directory"; value:":/plasma-workspace/timezone/offline_tiles" }
            PluginParameter { name:"osm.mapping.providersrepository.disabled"; value:true }
        }
        // Preserve the installed KDE offline map's tile/projection limits.
        map.minimumZoomLevel: 0
        map.center: surface.initialCenter && surface.initialCenter.length === 2
                    ? QtPositioning.coordinate(surface.initialCenter[0], surface.initialCenter[1]) : QtPositioning.coordinate(0, 0)
        map.zoomLevel: surface.initialZoom
        map.maximumZoomLevel: 4.90
        map.maximumTilt: 0
        map.onBearingChanged: if (view.map.bearing !== 0) view.map.bearing = 0
        map.onSupportedMapTypesChanged: {
            if (!surface.shuttingDown && view.map.supportedMapTypes.length)
                view.map.activeMapType = view.map.supportedMapTypes[0]
        }
        MapItemView {
            parent: view.map
            autoFitViewport: false
            model: surface.visiblePolygons
            delegate: MapPolygon {
                id: polygon
                required property var modelData
                objectName: "fluff-timezone-polygon"
                readonly property string tzid: modelData?.tzid || ""
                readonly property bool selected: tzid.length > 0 && surface.selectedTimeZone === tzid
                geoShape: modelData?.shape || QtPositioning.polygon()
                color: Kirigami.Theme.highlightColor
                border.width: 2
                border.color: Qt.darker(color)
                opacity: selected ? 0.8 : tzid.length > 0 && surface.hoveredTimeZone === tzid ? 0.6 : 0.1
                autoFadeIn: false
                referenceSurface: QtLocation.ReferenceSurface.Map
                TapHandler {
                    enabled: polygon.tzid.length > 0 && !surface.shuttingDown
                    onTapped: surface.chooseTimezone(polygon.tzid)
                }
                HoverHandler {
                    enabled: polygon.tzid.length > 0 && !surface.shuttingDown
                    onHoveredChanged: {
                        if (hovered) surface.hoveredTimeZone = polygon.tzid
                        else if (surface.hoveredTimeZone === polygon.tzid) surface.hoveredTimeZone = ""
                    }
                }
            }
        }
        Row {
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            anchors.margins: Kirigami.Units.largeSpacing
            spacing: Kirigami.Units.smallSpacing
            MapZoomButton { zoomIn:true }
            MapZoomButton { zoomIn:false }
        }
    }
    component MapZoomButton: Controls.Button {
        required property bool zoomIn
        text: surface.mapText("@action:button",zoomIn ? "Zoom in" : "Zoom out")
        icon.name: zoomIn ? "zoom-in-map-symbolic" : "zoom-out-map-symbolic"
        display: Controls.AbstractButton.IconOnly
        enabled: !surface.shuttingDown && (zoomIn ? view.map.zoomLevel < view.map.maximumZoomLevel : view.map.zoomLevel > view.map.minimumZoomLevel)
        onClicked: view.map.zoomLevel += zoomIn ? 0.5 : -0.5
        onDoubleClicked: view.map.zoomLevel += zoomIn ? 0.5 : -0.5
        Controls.ToolTip.text: text
        Controls.ToolTip.visible: hovered
        Controls.ToolTip.delay: Kirigami.Units.toolTipDelay
    }
    Component.onDestruction: shuttingDown = true
}
