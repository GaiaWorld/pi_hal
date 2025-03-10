use parry2d::bounding_volume::Aabb;

/// 二维空间中的点类型别名（使用f32精度）
type Point = parry2d::math::Point<f32>;
/// 二维向量类型别名（使用f32精度）
type Vector2 = parry2d::math::Vector<f32>;

/// 误差函数近似实现（最大误差小于0.00035）
/// 基于多项式近似公式：erf(x) ≈ 1 - 1/(1 + a1x + a2x² + a3x³ + a4x⁴)^4
/// 参数:
/// - x: 输入值
/// 返回: 误差函数计算结果，范围[-1, 1]
fn erf(mut x: f32) -> f32 {
    let negative = x < 0.0;
    if negative {
        x = -x;
    }

    let x2 = x * x;
    let x3 = x2 * x;
    let x4 = x2 * x2;
    let denom = 1.0 + 0.278393 * x + 0.230389 * x2 + 0.000972 * x3 + 0.078108 * x4;
    let result = 1.0 - 1.0 / (denom * denom * denom * denom);
    return if negative { -result } else { result };
}

/// 基于标准差调整的误差函数
/// 将输入值x按标准差sigma缩放后计算误差函数
/// 数学公式: erf(x/(σ√2))
/// 参数:
/// - x: 原始距离值
/// - sigma: 高斯分布的标准差
/// 返回: 缩放后的误差函数值
fn erf_sigma(x: f32, sigma: f32) -> f32 {
    return erf(x / (sigma * 1.4142135623730951));
}

/// 计算矩形区域的高斯模糊颜色值
/// 通过误差函数计算二维高斯积分，得到矩形区域的模糊强度
/// 数学公式: 
/// 1/4 * [erf_sigma(p1.x) - erf_sigma(p0.x)] * [erf_sigma(p1.y) - erf_sigma(p0.y)]
/// 参数:
/// - p0: 到矩形左上角的向量距离
/// - p1: 到矩形右下角的向量距离
/// - sigma: 高斯模糊的标准差
/// 返回: 归一化的模糊强度值，范围[0.0, 1.0]
fn color_from_rect(p0: Vector2, p1: Vector2, sigma: f32) -> f32 {
    return (erf_sigma(p1.x, sigma) - erf_sigma(p0.x, sigma))
        * (erf_sigma(p1.y, sigma) - erf_sigma(p0.y, sigma))
        / 4.0;
}

/// 计算指定位置点的阴影透明度
/// 参数:
/// - pos: 当前像素点的位置
/// - pt_min: 包围盒最小点（左上角）
/// - pt_max: 包围盒最大点（右下角）
/// - sigma: 高斯模糊的标准差
/// 返回: 该点的阴影透明度值，范围[0.0, 1.0]
fn get_shadow_alpha(pos: Point, pt_min: &Point, pt_max: &Point, sigma: f32) -> f32 {
    // Compute the vector distances 'p_0' and 'p_1'.
    let d_min = pos - pt_min;
    let d_max = pos - pt_max;

    // Compute the basic color '"colorFromRect"_sigma(p_0, p_1)'. This is all we have to do if
    // the box is unrounded.
    return color_from_rect(d_min, d_max, sigma);
}

/// 生成模糊后的像素图
/// 遍历每个像素位置，计算其阴影透明度并转换为8位灰度值
/// 参数:
/// - info: 包含模糊参数的BoxInfo结构体
/// 返回: 灰度像素数组，每个元素范围[0, 255]
pub fn blur_box(info: BoxInfo) -> Vec<u8> {
    let BoxInfo {
        p_w,
        p_h,
        start,
        px_dsitance,
        sigma,
        bbox,
        ..
    } = info;
    let mut pixmap = vec![0; (p_w * p_h) as usize];
    let start = Point::new(0.5, 0.5);
    for i in 0..p_w as usize {
        for j in 0..p_h as usize {
            let pos: parry2d::na::OPoint<f32, parry2d::na::Const<2>> =
                Point::new(start.x + i as f32, start.y + j as f32);
            let a = get_shadow_alpha(pos, &bbox.mins, &bbox.maxs, sigma);

            pixmap[j * p_w as usize + i as usize] = (a * 255.0) as u8;
        }
    }

    pixmap
}

/// 模糊处理参数结构体
/// 字段说明:
/// - p_w: 像素图宽度
/// - p_h: 像素图高度
/// - start: 起始点坐标（包围盒左上角偏移量）
/// - px_dsitance: 像素间距（与纹理尺寸相关）
/// - sigma: 高斯模糊标准差
/// - atlas_bounds: 纹理图集边界包围盒
/// - bbox: 实际使用的包围盒
/// - radius: 模糊半径
#[derive(Debug, Clone)]
pub struct BoxInfo {
    pub p_w: f32,
    pub p_h: f32,
    start: Point,
    px_dsitance: f32,
    sigma: f32,
    pub atlas_bounds: Aabb,
    bbox: Aabb,
    pub radius: u32,
}

/// 计算模糊框的布局参数
/// 参数:
/// - bbox: 原始包围盒
/// - txe_size: 纹理尺寸
/// - radius: 模糊半径
/// 返回: 包含布局参数的BoxInfo结构体
pub fn compute_box_layout(bbox: Aabb, txe_size: usize, radius: u32) -> BoxInfo {
    let b_w = bbox.maxs.x - bbox.mins.x;
    let b_h = bbox.maxs.y - bbox.mins.y;

    let px_dsitance = b_h.max(b_w) / (txe_size - 1) as f32; // 两边pxrange + 0.5， 中间应该减一

    // let px_num = (sigma + sigma * 5.0).ceil();
    let px_num = radius as f32;
    let px_num2 = px_num + 0.5;
    let sigma = px_num / 2.0;
    let dsitance = px_dsitance * px_num;
    println!("{:?}", (b_w, b_h, px_dsitance, px_num, dsitance, bbox));
    let p_w = (b_w / px_dsitance).ceil() + px_num2 * 2.0;
    let p_h = (b_h / px_dsitance).ceil() + px_num2 * 2.0;
    // let mut pixmap = vec![0; (p_w * p_h) as usize];
    println!("{:?}", (p_w, p_h));
    let start = Point::new(bbox.mins.x - dsitance, bbox.mins.y - dsitance);

    let maxs = if b_h > b_w {
        Point::new(b_w / px_dsitance + px_num2, p_h - px_num2)
    } else {
        Point::new(p_w - px_num2, b_h / px_dsitance + px_num2)
    };

    let atlas_bounds = Aabb::new(Point::new(px_num2, px_num2), maxs);
    let info = BoxInfo {
        p_w,
        p_h,
        start,
        px_dsitance,
        sigma,
        atlas_bounds,
        bbox: atlas_bounds,
        radius,
    };
    info
}
const SCALE: f32 = 10.0;
/// 执行高斯模糊处理
/// 参数:
/// - sdf_tex: 输入的有符号距离场纹理数据
/// - width: 纹理宽度
/// - height: 纹理高度
/// - radius: 模糊半径
/// - weight: 距离场权重系数（控制模糊强度）
/// 返回: 模糊后的灰度像素数组
pub fn gaussian_blur(
    sdf_tex: Vec<u8>,
    width: u32,
    height: u32,
    radius: u32,
    weight: f32,
) -> Vec<u8> {
    // let (width, height) = img.dimensions();
    let mut output = Vec::with_capacity(sdf_tex.len());
    let weight = -weight / SCALE;
    let kernel = create_gaussian_kernel(radius);
    let kernel_size = kernel.len() as u32;

    for y in 0..height {
        for x in 0..width {
            // let mut r = 0.0;
            // let mut g = 0.0;
            // let mut b = 0.0;
            let mut a = 0.0;
            let mut weight_sum = 0.0;

            for ky in 0..kernel_size {
                for kx in 0..kernel_size {
                    let px =
                        (x as i32 + kx as i32 - radius as i32).clamp(0, width as i32 - 1) as u32;
                    let py =
                        (y as i32 + ky as i32 - radius as i32).clamp(0, height as i32 - 1) as u32;

                    let sdf = sdf_tex[(px + py * width) as usize] as f32 / 255.0;
                    let fill_sd_px = sdf - (0.5 + weight);
                    let pixel = (fill_sd_px + 0.5).clamp(0.0, 1.0);

                    let weight = kernel[ky as usize][kx as usize];

                    // r += pixel[0] as f32 * weight;
                    // g += pixel[1] as f32 * weight;
                    // b += pixel[2] as f32 * weight;
                    a += pixel as f32 * weight;
                    weight_sum += weight;
                }
            }

            let pixel = (a / weight_sum * 255.0) as u8;

            output.push(pixel);
        }
    }

    output
}

/// 创建高斯卷积核
/// 根据半径生成二维高斯分布权重矩阵
/// 参数:
/// - radius: 模糊半径（决定核尺寸为 2r+1 x 2r+1）
/// 返回: 归一化的高斯核矩阵
fn create_gaussian_kernel(radius: u32) -> Vec<Vec<f32>> {
    let sigma = radius as f32 / 2.0;
    let size = radius * 2 + 1;
    let mut kernel = vec![vec![0.0; size as usize]; size as usize];
    let mut sum = 0.0;

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - radius as f32;
            let dy = y as f32 - radius as f32;
            let value = (-((dx * dx + dy * dy) / (2.0 * sigma * sigma))).exp()
                / (2.0 * std::f32::consts::PI * sigma * sigma);
            kernel[y as usize][x as usize] = value;
            sum += value;
        }
    }

    for y in 0..size {
        for x in 0..size {
            kernel[y as usize][x as usize] /= sum;
        }
    }

    kernel
}
