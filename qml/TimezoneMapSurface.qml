import QtQuick
import QtQuick.Controls as Controls
import QtLocation
import QtPositioning
import QtCore
import org.kde.kirigami as Kirigami
import org.kde.plasma.workspace.timezoneselector as Workspace

Item {
    id: surface
    // Keep native geographic shapes within the assigned map surface, including
    // when a scaled Loader hosts the map in the compact installer layout.
    layer.enabled: true
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
        .filter(row => row.tzid !== "UTC" && !row.tzid.startsWith("Etc/"))
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
        const next = polygons.filter(row => row.tzid === selectedTimeZone || intersects(row.bounds, renderBounds))
        // Do not rebuild all delegates when a pan/zoom leaves the same areas
        // in view. Besides wasted work, asynchronous rebuilds leave input holes.
        if (next.length !== visiblePolygons.length || next.some((row,index) => row !== visiblePolygons[index]))
            visiblePolygons = next
    }
    onPolygonsChanged: updateViewport()
    onSelectedTimeZoneChanged: viewportTimer.start()
    Timer {
        id: viewportTimer
        // start(), unlike restart(), keeps refreshing during a continuous
        // pan/zoom instead of waiting until the gesture has stopped.
        interval: 80
        onTriggered: surface.updateViewport()
    }
    Connections {
        target: view.map
        function onCenterChanged() { viewportTimer.start() }
        function onZoomLevelChanged() { viewportTimer.start() }
        function onWidthChanged() { viewportTimer.start() }
        function onHeightChanged() { viewportTimer.start() }
        function onMapReadyChanged() { viewportTimer.start() }
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
        return refineEgyptBorder(joinJerusalemAreas(rows))
    }
    function refineEgyptBorder(rows) {
        // Only replace KDE's known coarse shared edge. Newer installed geometry
        // that no longer has this chain takes precedence over this local fix.
        const coarse = [[31.466843,34.054691],[31.220129,34.267499],[29.45338,34.921211]]
        // Natural Earth v5.1.2, 10m admin-0 boundary lines, public domain:
        // ne_id 1746707921 (coast to Rafah), then 1746707915 (Rafah to Taba).
        // https://github.com/nvkelso/natural-earth-vector/blob/v5.1.2/geojson/ne_10m_admin_0_boundary_lines_land.geojson
        const boundary = [
            [31.314267,34.200269],[31.275715,34.224608],[31.25923,34.230396],
            [31.211449,34.248351],[31.184144,34.258611],[31.078776,34.297989],
            [30.994492,34.329511],[30.907417,34.367855],[30.7913,34.418808],
            [30.651205,34.480407],[30.571675,34.502214],[30.530334,34.504384],
            [30.513332,34.510379],[30.494574,34.526295],[30.494522,34.526295],
            [30.482172,34.536217],[30.468581,34.53601],[30.438712,34.526502],
            [30.421142,34.524745],[30.409618,34.526915],[30.400213,34.53384],
            [30.35882,34.58841],[30.344506,34.599469],[30.26203,34.632645],
            [30.114545,34.69166],[30.018169,34.730417],[30.012588,34.733208],
            [30.006697,34.734965],[30.000702,34.735585],[29.994553,34.735068],
            [29.940241,34.741373],[29.835699,34.785194],[29.7417,34.824365],
            [29.63876,34.8501],[29.569643,34.848239],[29.545717,34.855267],
            [29.504298,34.878108],[29.490058,34.886729]
        ].map(point => QtPositioning.coordinate(...point))
        function same(point, pair) {
            return Math.abs(point.latitude-pair[0]) < 1e-8 && Math.abs(point.longitude-pair[1]) < 1e-8
        }
        function edge(row) {
            if (row.shape.holesCount()) return null
            const path = row.coordinates.slice()
            if (path.length > 1 && path[0].latitude === path[path.length-1].latitude
                    && path[0].longitude === path[path.length-1].longitude) path.pop()
            for (const reversed of [false,true]) {
                const chain = reversed ? coarse.slice().reverse() : coarse
                for (let index = 0; index < path.length; ++index)
                    if (chain.every((pair,offset) => same(path[(index+offset)%path.length],pair)))
                        return {path,index,reversed}
            }
            return null
        }
        const jerusalem = rows.find(row => row.tzid === "Asia/Jerusalem" && edge(row))
        const cairo = rows.find(row => row.tzid === "Africa/Cairo" && edge(row))
        // Apply atomically: never move just one side of a shared boundary.
        if (!jerusalem || !cairo) return rows
        return rows.map(row => {
            if (row !== jerusalem && row !== cairo) return row
            const match = edge(row)
            const coordinates = match.reversed ? boundary.slice().reverse() : boundary.slice()
            for (let offset = coarse.length; offset < match.path.length; ++offset)
                coordinates.push(match.path[(match.index+offset)%match.path.length])
            return Object.assign({},row,{coordinates,shape:QtPositioning.polygon(coordinates),
                bounds:QtPositioning.rectangle(coordinates)})
        })
    }
    function joinJerusalemAreas(rows) {
        // This is FluffSetup's local display policy, not a tzdata alias. Keep
        // native vertices, but remove the two independent selectable areas and
        // dissolve their shared/internal outlines into the Jerusalem area.
        const ids = ["Asia/Jerusalem", "Asia/Gaza", "Asia/Hebron"]
        const combined = rows.filter(row => ids.includes(row.tzid))
        if (!combined.length) return rows
        function same(a, b) { return a.latitude === b.latitude && a.longitude === b.longitude }
        function ring(row) {
            const path = row.coordinates.slice()
            if (path.length > 1 && same(path[0], path[path.length - 1])) path.pop()
            return path
        }
        function boundaryContains(path, point) {
            for (let index = 0; index < path.length; ++index) {
                const a = path[index], b = path[(index + 1) % path.length]
                const cross = (point.longitude - a.longitude) * (b.latitude - a.latitude)
                    - (point.latitude - a.latitude) * (b.longitude - a.longitude)
                if (Math.abs(cross) < 1e-10 && point.longitude >= Math.min(a.longitude, b.longitude) - 1e-10
                        && point.longitude <= Math.max(a.longitude, b.longitude) + 1e-10
                        && point.latitude >= Math.min(a.latitude, b.latitude) - 1e-10
                        && point.latitude <= Math.max(a.latitude, b.latitude) + 1e-10) return true
            }
            return false
        }
        function contained(inner, outer) {
            if (inner.shape.holesCount() || outer.shape.holesCount()) return false
            const path = ring(inner), boundary = ring(outer)
            return path.every((point, index) => {
                const next = path[(index + 1) % path.length]
                const middle = QtPositioning.coordinate((point.latitude + next.latitude) / 2,
                    (point.longitude + next.longitude) / 2)
                return (outer.shape.contains(point) || boundaryContains(boundary, point))
                    && (outer.shape.contains(middle) || boundaryContains(boundary, middle))
            })
        }
        function merged(a, b) {
            // Preserve any future holes/overlaps that cannot be safely joined;
            // never replace the real area with a convex hull or a guessed border.
            if (a.shape.holesCount() || b.shape.holesCount()) return null
            const left = ring(a), right = ring(b)
            for (let i = 0; i < left.length; ++i) for (let j = 0; j < right.length; ++j) {
                let other = right, edge = j
                if (same(left[i], right[j]) && same(left[(i + 1) % left.length], right[(j + 1) % right.length])) {
                    other = right.slice().reverse()
                    edge = right.length - 2 - j
                    if (edge < 0) edge += right.length
                }
                if (!same(left[i], other[(edge + 1) % other.length])
                        || !same(left[(i + 1) % left.length], other[edge])) continue
                const coordinates = []
                for (let step = 1; step <= left.length; ++step)
                    coordinates.push(left[(i + step) % left.length])
                for (let step = 2; step < other.length; ++step)
                    coordinates.push(other[(edge + step) % other.length])
                const shape = QtPositioning.polygon(coordinates)
                const row = {tzid:"Asia/Jerusalem", shape, coordinates,
                    bounds:QtPositioning.rectangle(coordinates)}
                // Adjacent edges must expand the area, never cut either piece.
                if (contained(a, row) && contained(b, row)) return row
            }
            return null
        }
        const pieces = []
        for (const row of combined) {
            if (pieces.some(other => contained(row, other))) continue
            for (let index = pieces.length - 1; index >= 0; --index)
                if (contained(pieces[index], row)) pieces.splice(index, 1)
            pieces.push(Object.assign({}, row, {tzid:"Asia/Jerusalem"}))
        }
        for (let i = 0; i < pieces.length; ++i) for (let j = i + 1; j < pieces.length; ++j) {
            const joined = merged(pieces[i], pieces[j])
            if (!joined) continue
            pieces[i] = joined
            pieces.splice(j, 1)
            i = -1 // Recheck all pieces after removing a shared edge.
            break
        }
        const result = []
        let inserted = false
        for (const row of rows) {
            if (!ids.includes(row.tzid)) result.push(row)
            else if (!inserted) {
                result.push(...pieces)
                inserted = true
            }
        }
        return result
    }
    function chooseTimezone(id) {
        if (!shuttingDown && validTimeZones.includes(id) && availableMapTimeZones.includes(id))
            zoneSelected(id)
    }
    function timezoneAt(coordinate) {
        let selected = "", smallestBounds = Infinity
        // Native data can overlap: Urumqi sits inside Shanghai's wider area.
        // Resolve one tap once, preferring the more specific bounding area,
        // rather than letting several polygon handlers overwrite the choice.
        for (const row of visiblePolygons) {
            if (!row.bounds.contains(coordinate) || !row.shape.contains(coordinate)
                    || !validTimeZones.includes(row.tzid)) continue
            const area = row.bounds.width * row.bounds.height
            if (area < smallestBounds) {
                selected = row.tzid
                smallestBounds = area
            }
        }
        if (selected.length) return selected
        // At a world-scale zoom, a small island can be less than one input
        // pixel high. Rounding a pointer coordinate can then miss even its
        // center. Allow one pixel around only these tiny native bounds; exact
        // geographic hits always win and larger areas keep their real border.
        const pixels = 256 * Math.pow(2, view.map.zoomLevel)
        const point = view.map.fromCoordinate(coordinate, false)
        let nearest = Infinity
        for (const row of visiblePolygons) {
            if (!validTimeZones.includes(row.tzid)) continue
            const width = row.bounds.width * pixels / 360
            if (width > 4) continue
            const top = QtPositioning.coordToMercator(row.bounds.topLeft)
            const bottom = QtPositioning.coordToMercator(row.bounds.bottomRight)
            const height = Math.abs(bottom.y - top.y) * pixels
            if (height > 4) continue
            const center = view.map.fromCoordinate(row.bounds.center, false)
            const dx = Math.max(0, Math.abs(point.x - center.x) - width / 2)
            const dy = Math.max(0, Math.abs(point.y - center.y) - height / 2)
            const distance = dx * dx + dy * dy
            if (distance <= 1 && distance < nearest) {
                selected = row.tzid
                nearest = distance
            }
        }
        return selected
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
        map.clip: true
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
        TapHandler {
            parent: view.map
            enabled: !surface.shuttingDown
            onTapped: eventPoint => surface.chooseTimezone(surface.timezoneAt(view.map.toCoordinate(eventPoint.position, false)))
        }
        HoverHandler {
            parent: view.map
            enabled: !surface.shuttingDown && parent.enabled
            onPointChanged: if (hovered) surface.hoveredTimeZone = surface.timezoneAt(view.map.toCoordinate(point.position, false))
            onHoveredChanged: surface.hoveredTimeZone = hovered
                ? surface.timezoneAt(view.map.toCoordinate(point.position, false)) : ""
        }
        MapItemView {
            parent: view.map
            autoFitViewport: false
            // Qt otherwise creates a wide view's areas over multiple frames,
            // leaving some visible regions undrawn and unclickable meanwhile.
            incubateDelegates: false
            // Disable removal animation entirely, not an empty animation group.
            remove: null
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
    // Geometry and the camera may already be ready before Connections attach.
    // An unselected multi-zone region can keep that initial camera unchanged,
    // so do not rely on a later zoom/pan signal to populate its visible areas.
    Component.onCompleted: viewportTimer.start()
    Component.onDestruction: shuttingDown = true
}
