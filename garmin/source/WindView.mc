import Toybox.WatchUi;

class WindView extends WatchUi.View {
    var boat;

    function initialize(boat as Boat) {
        View.initialize();
        self.boat = boat;
    }

    function onUpdate(dc) {
        View.onUpdate(dc);
        dc.setAntiAlias(true);

        var w = dc.getWidth();
        var h = dc.getHeight();

        var cx = w / 2;
        var cy = h / 2;

        dc.setPenWidth(cx * 0.1);
        dc.setColor(GREEN, Graphics.COLOR_BLACK);
        dc.drawArc(cx, cy, cx, Graphics.ARC_COUNTER_CLOCKWISE, 30, 70);
        dc.setColor(RED, Graphics.COLOR_BLACK);
        dc.drawArc(cx, cy, cx, Graphics.ARC_COUNTER_CLOCKWISE, 110, 150);

        dc.setPenWidth(5);
        for (var i = 0; i < 24; i++) {
            var t = i / 24.0;
            var θ = t * TAU;
            var x = Math.cos(θ + PI_FRAC_2);
            var y = Math.sin(θ + PI_FRAC_2);

            if (i % 2 == 0) {
                dc.setColor(Graphics.COLOR_WHITE, Graphics.COLOR_BLACK);
                var degree = Math.round(Math.toDegrees(θ - Math.PI))
                    .toNumber()
                    .abs();
                if (degree <= 90) {
                    dc.drawText(
                        cx + x * cx * 0.8,
                        cy + y * cy * 0.8,
                        Graphics.FONT_XTINY,
                        degree,
                        Graphics.TEXT_JUSTIFY_CENTER |
                            Graphics.TEXT_JUSTIFY_VCENTER
                    );
                }

                dc.drawLine(
                    cx + x * cx,
                    cy + y * cy,
                    cx + x * cx * 0.95,
                    cy + y * cy * 0.95
                );
            } else {
                dc.setColor(Graphics.COLOR_LT_GRAY, Graphics.COLOR_BLACK);
                dc.drawLine(
                    cx + x * cx,
                    cy + y * cy,
                    cx + x * cx * 0.98,
                    cy + y * cy * 0.98
                );
            }
        }

        dc.setColor(Graphics.COLOR_WHITE, Graphics.COLOR_BLACK);
        var gap = 30.0;
        dc.drawText(
            cx - gap,
            h * .7,
            fontBolt(100),
            boat.speed.format("%.1f"),
            Graphics.TEXT_JUSTIFY_RIGHT | Graphics.TEXT_JUSTIFY_VCENTER
        );
        dc.drawText(
            cx - gap,
            h * .7 + 45,
            fontBolt(20),
            "KTS",
            Graphics.TEXT_JUSTIFY_RIGHT | Graphics.TEXT_JUSTIFY_VCENTER
        );

        dc.drawText(
            cx + gap,
            h * .7,
            fontBolt(100),
            boat.wind_speed.format("%.1f"),
            Graphics.TEXT_JUSTIFY_LEFT | Graphics.TEXT_JUSTIFY_VCENTER
        );
        dc.drawText(
            cx + gap,
            h * .7 + 45,
            fontBolt(20),
            "KTS",
            Graphics.TEXT_JUSTIFY_LEFT | Graphics.TEXT_JUSTIFY_VCENTER
        );

        var x = Math.cos(boat.wind_angle - PI_FRAC_2);
        var y = Math.sin(boat.wind_angle - PI_FRAC_2);
        var r = 6.0;
        var l = cx * 0.9;
        dc.fillPolygon([
            [cx + x * l, cy + y * l],
            [cy + y * r, cx - x * r],
            [cy - y * r, cx + x * r],
        ]);
        dc.fillCircle(cx, cy, r * 2);
        dc.setColor(DARK_GRAY, Graphics.COLOR_BLACK);
        dc.drawCircle(cx, cy, r * 2);
    }
}