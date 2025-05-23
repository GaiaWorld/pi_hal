export class AtomTable{
	stringMap /*Map<number, string>*/ = new Map();
	numberMap /*Map<string, number>*/ = new Map();

	get_string(k) {
		return this.stringMap.get(k)
	}

	get_number(k) {
		return this.numberMap.get(k)
	}

	set(n, s) {
		this.stringMap.set(n, s);
		this.numberMap.set(s, n);
	}
}

export const atomTable = new AtomTable();

export function fillBackGround(canvas, ctx, x, y) {
	canvas.width = x;
	canvas.height = y;
	ctx.fillStyle = "#00f";
	ctx.fillRect(0, 0, canvas.width, canvas.height);
}

export function drawCharWithStroke(ctx, ch_code, x, y) {
	var ch = String.fromCharCode(ch_code);
	//fillText 和 strokeText 的顺序对最终效果会有影响， 为了与css text-stroke保持一致， 应该fillText在前
	ctx.strokeText(ch, x, y);
	ctx.fillText(ch, x, y);
}

export function drawChar(ctx, ch_code, x, y) {
	var ch = String.fromCharCode(ch_code);
	//fillText 和 strokeText 的顺序对最终效果会有影响， 为了与css text-stroke保持一致， 应该fillText在前
    ctx.fillText(ch, x, y);
}

export function setFont(ctx, weight, fontSize, font/*number */, strokeWidth) {
	font = atomTable.get_string(font);
	var weight;
	if (weight <= 300 ) {
		weight = "lighter";
	} else if (weight < 700 ) {
		weight = "normal";
	} else if (weight < 900 ) {
		weight = "bold";
	} else {
		weight = "bolder";
	}
	ctx.font = weight + " " + fontSize + "px " + font;
	ctx.fillStyle = "#0f0";
	ctx.textBaseline = "top";

	if(strokeWidth > 0) {
		ctx.lineWidth = strokeWidth;
		ctx.strokeStyle = "#f00";
	}
}

export function useVao() {
	var u = navigator.userAgent.toLowerCase(); 
	return u.indexOf("ipad") < 0 && u.indexOf("iphone") < 0;
}

export function measureText(ctx, ch, font_size, name/**number*/) {
	name = atomTable.get_string(name);
	ctx.font = font_size + "px " + name;
	return ctx.measureText(String.fromCharCode(ch)).width;
}

export function loadImage(image_name, callback) {
	var image = new Image();
	image.onload = function() {
		callback(image_name.endsWith("png")?1:0, -1, 0, image_name, image.width, image.height, image);
	};
	image.src = image_name;
}

export function getGlobalMetricsHeight(font_name, size) {
	return 0.0
}



export function loadFile(image_name) {
	return Promise.resolve(new Uint8Array(0))
}
export function loadImageAsCanvas(image_name) {
	return Promise.resolve(new Uint8Array(0))
}

export function hasAtom(key) {

}

export function setAtom(key, v) {

}

export function initLocalStore()  {
	// return Promise.resolve(new Uint8Array(0))
};

/**
 * 从indexDb读数据
 */
// tslint:disable-next-line:no-reserved-keywords
export function get (key)  {
	return Promise.resolve(new Uint8Array(0))
};

/**
 * 往indexDb写数据
 */
export function write  (key, data)  {
	// return Promise.resolve()
};

/**
 * 从indexDb删除数据
 */
export function deleteKey (key) {
	// return Promise.resolve()
};

export function createFace(fnot_data)  {}

export async function computerSdf(char, outline) {}

export async function computeSdfTex(outline, size, pxrange){}

export function horizontalAdvance(face, text) {}
export function horizontalAdvanceOfGlyphIndex(face, glyph_index) {}

export function ascender(face) {}

export function descender(face) {}

export function maxBox(face) {}

export function maxBoxNormaliz(face) {}

export function toOutline(face, text) {}
export function toOutlineOfGlyphIndex(face, glyph_index) {}

export function toOutline3(face, text) {}
export function glyphIndex(face, text) {}
export function glyphIndexs(face, text, script) {}

export function debugSize(face) {}

export function createCircle(cx, cy, radius)  {}

export function createRect(x, y, width, height) {}

export function createSegment(ax, ay, bx, by) {}

export function createEllipse(cx, cy, rx, ry) {}

export function createPolygon(points){}

export function createPolyline(points)  {}

export function createPath(verb, points)  {}

export function getSvgInfo(shape){}

export function computerSvgSdf(svg_info){}

export async function computeShapeSdfTex(info, size, pxrange, is_outer_glow, cur_off){}

export async function computeArcsSdfTex(info, size, pxrange) {}

export function free(obj){}

export function loadFontSdf() {
	return Promise.resolve(new Uint8Array(0))
}
export function computeSdfCellOfWasm(info, scale){}
