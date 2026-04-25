import Toybox.Graphics;
import Toybox.Lang;
import Toybox.Math;

const TAU = Math.PI * 2.0;
const PI_FRAC_2 = Math.PI / 2.0;
const PI_2_FRAC_3 = (Math.PI * 2.0) / 3.0;

const MPS_TO_KNOTS = 1.944;

const RED = Graphics.createColor(255, 0xe7, 0x25, 0x2e);
const GREEN = Graphics.createColor(255, 0x06, 0xa8, 0x4f);
const DARK_GRAY = Graphics.createColor(255, 0x32, 0x32, 0x32);

public class Const {
}

function fontBolt(size as Number) as Graphics.VectorFont {
    return Graphics.getVectorFont({
        :face => ["RobotoCondensedBold"],
        :size => size,
    });
}

function fontRegular(size as Number) as Graphics.VectorFont {
    return Graphics.getVectorFont({
        :face => ["RobotoRegular"],
        :size => size,
    });
}
