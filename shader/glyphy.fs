#version 450

#extension GL_OES_standard_derivatives : enable

precision highp float;

#define GLYPHY_INFINITY 1e6
#define GLYPHY_EPSILON  1e-4
#define GLYPHY_MAX_D 0.5
#define GLYPHY_MAX_NUM_ENDPOINTS 20


// (max_offset, min_sdf, sdf_step, check)
// 如果 晶格的 sdf 在 [-check, check]，该晶格 和 字体轮廓 可能 相交 


layout(set = 0, binding = 0) uniform sampler index_tex_samp;
layout(set = 0, binding = 1) uniform texture2D u_index_tex;

layout(set = 1, binding = 0) uniform sampler data_tex_samp;
layout(set = 1, binding = 1) uniform texture2D u_data_tex;
layout(set = 1, binding = 2) uniform vec4 data_tex_size_buffer;

layout (location = 0) in vec2 uv; // uv坐标
layout (location = 1) in vec4 u_info; // sdf附加数据

out vec4 fragColor;
// out float sdf;

// 索引信息  
struct glyphy_index_t {
	
	// 编码信息
	int encode;

	// 端点的数量 
	// 0 代表 一直读取到 像素为 (0, 0, 0, 0) 的 数据为止
	int num_endpoints;

	// 在数据纹理的偏移，单位：像素
	int offset;

	// 晶格中心点的sdf
	float sdf;
};

// 从 p0 到 p1 的 圆弧
// 2 * d 为 tan(弧心角)
// d = 0 代表 这是 一条线段 
struct glyphy_arc_t {
	vec2  p0;
	vec2  p1;
	float d;
};

// 圆弧 端点 
struct glyphy_arc_endpoint_t {
	// 圆弧 第二个 端点 
	vec2  p;
	
	/** 
	 * d = 0 表示 这是一个 line 
	 * d = Infinity 表示 该点是 move_to 语义，通过 glyphy_isinf() 判断 
	 */
	float d;
};

struct line_t {
	float distance;

	float angle;
};

// 修复glsl bug 的 取余
// 某些显卡, 当b为uniform, 且 a % b 为 0 时候，会返回 b
// 128 , 256
vec2 div_mod(float a, float b) {
	float d = floor(a / b);
	float m = mod(a, b);
	if (m == b) {
		return vec2(d + 1.0, 0.0);
	}
	return vec2(d, m);
}

// 超过 最大值的 一半，就是 无穷 
bool glyphy_isinf(const float v)
{
	return abs (v) >= GLYPHY_INFINITY * 0.5;
}

// 小于 最小值 的 两倍 就是 0 
bool glyphy_iszero(const float v)
{
	return abs (v) <= GLYPHY_EPSILON * 2.0;
}

// v 的 垂直向量 
vec2 glyphy_ortho(const vec2 v)
{
	return vec2 (-v.y, v.x);
}

// [0, 1] 浮点 --> byte 
int glyphy_float_to_byte(const float v)
{
	return int (v * (256.0 - GLYPHY_EPSILON));
}

// [0, 1] 浮点 --> byte 
ivec4 glyphy_vec4_to_bytes(const vec4 v)
{
	return ivec4 (v * (256.0 - GLYPHY_EPSILON));
}

// 浮点编码，变成两个 整数 
ivec2 glyphy_float_to_two_nimbles(const float v)
{
	int f = glyphy_float_to_byte (v);


	vec2 r = div_mod(float(f), 16.0);

	return ivec2 (f / 16, int(r.y));
}

// returns tan (2 * atan (d))
float glyphy_tan2atan(const float d)
{
	return 2.0 * d / (1.0 - d * d);
}

// 取 arc 的 圆心 
vec2 glyphy_arc_center(const glyphy_arc_t a)
{
	return mix (a.p0, a.p1, 0.5) +
		glyphy_ortho(a.p1 - a.p0) / (2.0 * glyphy_tan2atan(a.d));
}

float glyphy_arc_wedge_signed_dist_shallow(const glyphy_arc_t a, const vec2 p)
{
	vec2 v = normalize (a.p1 - a.p0);
	float line_d = dot (p - a.p0, glyphy_ortho (v));
	if (a.d == 0.0) {
		return line_d;
	}
	
	float d0 = dot ((p - a.p0), v);
	if (d0 < 0.0) {
		return sign (line_d) * distance (p, a.p0);
	}

	float d1 = dot ((a.p1 - p), v);
	if (d1 < 0.0) {
		return sign (line_d) * distance (p, a.p1);
	}
	
	float r = 2.0 * a.d * (d0 * d1) / (d0 + d1);
	if (r * line_d > 0.0) {
		return sign (line_d) * min (abs (line_d + r), min (distance (p, a.p0), distance (p, a.p1)));
	}

	return line_d + r;
}

float glyphy_arc_wedge_signed_dist(const glyphy_arc_t a, const vec2 p)
{
	if (abs (a.d) <= 0.03) {
		return glyphy_arc_wedge_signed_dist_shallow(a, p);
	}
	
	vec2 c = glyphy_arc_center (a);
	return sign (a.d) * (distance (a.p0, c) - distance (p, c));
}

// 解码 arc 端点 
glyphy_arc_endpoint_t glyphy_arc_endpoint_decode(const vec4 v, const vec2 nominal_size)
{
	vec2 p = (vec2 (glyphy_float_to_two_nimbles (v.a)) + v.gb) / 16.0;
	float d = v.r;
	if (d == 0.0) {
		d = GLYPHY_INFINITY;
	} else {
		d = float(glyphy_float_to_byte(d) - 128) * GLYPHY_MAX_D / 127.0;
	}

	p *= nominal_size;
	return glyphy_arc_endpoint_t (p, d);
}

// 判断是否 尖角内 
bool glyphy_arc_wedge_contains(const glyphy_arc_t a, const vec2 p)
{
	float d2 = glyphy_tan2atan (a.d);

	return dot (p - a.p0, (a.p1 - a.p0) * mat2(1,  d2, -d2, 1)) >= 0.0 &&
		dot (p - a.p1, (a.p1 - a.p0) * mat2(1, -d2,  d2, 1)) <= 0.0;
}

// 点 到 圆弧 的 距离
float glyphy_arc_extended_dist(const glyphy_arc_t a, const vec2 p)
{
	// Note: this doesn't handle points inside the wedge.
	vec2 m = mix(a.p0, a.p1, 0.5);

	float d2 = glyphy_tan2atan(a.d);

	if (dot(p - m, a.p1 - m) < 0.0) {
		return dot(p - a.p0, normalize((a.p1 - a.p0) * mat2(+d2, -1, +1, +d2)));
	} else {
		return dot(p - a.p1, normalize((a.p1 - a.p0) * mat2(-d2, -1, +1, -d2)));
	}
}

line_t decode_line(const vec4 v, const vec2 nominal_size) {
	ivec4 iv = glyphy_vec4_to_bytes(v);

	line_t l;

	int ua = iv.b * 256 + iv.a;
	int ia = ua - 0x8000;
	l.angle = -float(ia) / float(0x7FFF) * 3.14159265358979;

	int ud = (iv.r - 128) * 256 + iv.g;
	int id = ud - 0x4000;
	float d = float(id) / float(0x1FFF);
	
	float scale = max(nominal_size.x, nominal_size.y);
	
	l.distance = d * scale;
	return l;
}

// 解码 索引纹理 
glyphy_index_t decode_glyphy_index(vec4 v)
{	
	ivec4 c = glyphy_vec4_to_bytes(v);

	int value = c.r + 256 * c.g;

	int v2 = value;

	// 注：移动端，int 范围有可能是 [-2^15, 2^15)
	if (value < 0) {
		v2 += 32766;
		v2 += 2;
	}

	int num_endpoints = v2 / 16384;
	int sdf_and_offset_index = v2 - 16384 * num_endpoints;
	if (value < 0) {
		num_endpoints += 2; // 因为 32768 / 16384 = 2
	}

	// Amd 显卡 Bug：整除时，余数不为0
	if (sdf_and_offset_index == 16384) {
		sdf_and_offset_index = 0;
		num_endpoints += 1;
	}

	int sdf_index = sdf_and_offset_index / int(u_info.x);
	int offset = sdf_and_offset_index - sdf_index * int(u_info.x);
	
	// Amd 显卡 Bug：整除时，余数不为0；
	if (offset == int(u_info.x)) {
		offset = 0;
		sdf_index += 1;
	}
	
	float sdf = 0.0;

	if (sdf_index == 0) {
		// 用 0 表示 完全 在内 的 晶格！
		sdf = -GLYPHY_INFINITY;
	} else if (sdf_index == 1) {
		// 用 1 表示 完全 在外 的 晶格！
		sdf = GLYPHY_INFINITY;
	} else {
		// 比实际的 sdf 范围多出 2
		sdf_index -= 2;
		sdf = float(sdf_index) * u_info.z + u_info.y;
	}

	glyphy_index_t index;

	index.sdf = sdf;
	index.encode = v2;
	index.offset = offset;
	index.num_endpoints = num_endpoints;
	
	return index;
}

// 取 索引 uv
// vec2 get_index_uv(vec2 p)
// {
// 	vec2 offset = vec2(index_offset_and_size.xy);
// 	return (p + offset) / index_tex_size;
// }



glyphy_index_t get_glyphy_index(vec2 uv) {
	
	// vec2 index_uv = get_index_uv(p);
	
	vec4 c = texture(sampler2D(u_index_tex, index_tex_samp), uv).rgba;
	return decode_glyphy_index(c);
}


// 重点 计算 sdf 
float glyphy_sdf(vec2 uv, vec2 nominal_size) {
	vec2 p = uv * nominal_size;
	glyphy_index_t index_info = get_glyphy_index(uv);
		
	if (index_info.sdf >= GLYPHY_INFINITY - GLYPHY_EPSILON) {
		// 全外面
		return GLYPHY_INFINITY;
	} else if (index_info.sdf <= -GLYPHY_INFINITY + GLYPHY_EPSILON) {
		// 全里面
		return -GLYPHY_INFINITY;
	}

	// 处理相交的晶格

	float side = index_info.sdf < 0.0 ? -1.0 : 1.0;
	float min_dist = GLYPHY_INFINITY;
	
	// 注：N卡，和 高通 的 显卡，纹理 需要 加 0.5像素
	float offset = float(index_info.offset);
	// float a = offset / u_info.x;
	float x = offset / (data_tex_size_buffer.x - 0.5);
	float y = 0.5;


	vec4 rgba = texture(sampler2D(u_data_tex, data_tex_samp), vec2(x, y));
	

	glyphy_arc_t closest_arc;
	glyphy_arc_endpoint_t endpoint = glyphy_arc_endpoint_decode(rgba, nominal_size);

	
	vec2 pp = endpoint.p;
	// 1个像素 最多 32次 采样 
	for(int i = 1; i < GLYPHY_MAX_NUM_ENDPOINTS; i++) {
		// vec4 rgba = vec4(0.0);
		float offset = 0.5 + float(index_info.offset + i);
		float x = offset / (data_tex_size_buffer.x - 0.5);
		float y = 0.5;
		
		vec4 rgba = texture(sampler2D(u_data_tex, data_tex_samp), vec2(x, y) );

		if(index_info.num_endpoints == 0) {
			if (rgba == vec4(0.0)) {
				break;
			}
		} else if (i < index_info.num_endpoints) {
		} else {
			break;
		}
		
		endpoint = glyphy_arc_endpoint_decode(rgba, nominal_size);
		
		glyphy_arc_t a = glyphy_arc_t(pp, endpoint.p, endpoint.d);

		// 无穷的 d 代表 Move 语义 
		if(glyphy_isinf(a.d)) {
			pp = endpoint.p;
			continue;
		}

		if(glyphy_arc_wedge_contains(a, p)) { // 处理 尖角 
			float sdist = glyphy_arc_wedge_signed_dist(a, p);
			float udist = abs(sdist) * (1.0 - GLYPHY_EPSILON);

			if(udist <= min_dist) {
				min_dist = udist;
				side = sdist <= 0.0 ? -1.0 : +1.0;
			}
		} else {
			float udist = min(distance(p, a.p0), distance(p, a.p1));

			if(udist < min_dist - GLYPHY_EPSILON) {
				side = 0.0;
				min_dist = udist;
				closest_arc = a;
			} else if(side == 0.0 && udist - min_dist <= GLYPHY_EPSILON) {
				float old_ext_dist = glyphy_arc_extended_dist(closest_arc, p);
				float new_ext_dist = glyphy_arc_extended_dist(a, p);

				float ext_dist = abs(new_ext_dist) <= abs(old_ext_dist) ? old_ext_dist : new_ext_dist;

				side = sign(ext_dist);
			}
		}
		pp = endpoint.p;
	}
	
	if(side == 0.) {
		float ext_dist = glyphy_arc_extended_dist(closest_arc, p);
		side = sign(ext_dist);
	}

	// 线段 特殊处理
	if(index_info.num_endpoints == 1) {
		line_t line = decode_line(rgba, nominal_size);
		
		vec2 n = vec2(cos(line.angle), sin(line.angle));
		
		side = 1.0;
		
		// min_dist = float(index_info.num_endpoints) / 6.0;
		min_dist = dot(p - 0.5 * vec2(nominal_size), n) - line.distance;
	}

		// side = 1.0;
		// min_dist = float(index_info.num_endpoints) / 6.0;
	// }
 
	return min_dist * side;
}

// 1.0 / sqrt(2.0)
#define SQRT2_2 0.70710678118654757 

// sqrt(2.0)
#define SQRT2   1.4142135623730951

struct glyph_info_t {
	// 网格 宽度，高度 的 格子数量 
	vec2 nominal_size;

	// 索引纹理坐标
	vec2 atlas_pos;

	float sdf;
};

// 解码 
// v.x (有效位 低15位) --> (高7位:纹理偏移.x, 中6位:网格宽高.x, 低2位: 00) 
// v.y (有效位 低15位) --> (高7位:纹理偏移.y, 中6位:网格宽高.y, 低2位: 00) 
glyph_info_t glyph_info_decode(vec2 v) {
	glyph_info_t gi;

	// mod 256 取低8位
	// 除4 取低8位中的 高6位
	
	vec2 rx = div_mod(v.x, 256.0);
	vec2 ry = div_mod(v.y, 256.0);

	vec2 r = vec2(rx.y, ry.y);
	
	// TODO +2 不了解什么意思 
	ivec2 size = (ivec2(r) + 2) / 4;
	gi.nominal_size = vec2(size);

	// 去掉 低8位的 信息 
	ivec2 pos = ivec2(v) / 256;
	gi.atlas_pos = vec2(pos);
	
	return gi;
}

// 抗锯齿 1像素 
// d 在 [a, b] 返回 [0.0, 1.0] 
float antialias(float d) {
	// TODO 这个值，文字越少，就应该越少。否则 就会出现 模糊；
	float b = 0.3;
	float a = -b;

	float r = (-d - a) / (b - a);

	return clamp(r, 0.0, 1.0);
}

void main() {
	vec2 nominal_size = vec2(32., 32.);
	vec2 uv1 = vec2(uv.x, 1.0 - uv.y);
	vec2 p = uv1 * nominal_size;

	float pxrange =  data_tex_size_buffer.y;
	float is_area = data_tex_size_buffer.z;

	// 重点：计算 SDF 
	// 默认pxrange= 6.7 ; 0.5 范围内除以2为3.35
	float gsdist = glyphy_sdf(uv1, nominal_size) * (0.5 / pxrange);
	// 均匀缩放 
	float scale = SQRT2 / length(fwidth(p));
	gsdist = gsdist * scale;

	if (is_area < 0.5){
		gsdist = abs(gsdist);
	} 
	
	float d = 0.5 - gsdist;
	fragColor = vec4(d, d, d, 1.0);
}