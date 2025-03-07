//! 文字装箱算法（货架算法）
//! 
//! 用于管理文字在纹理图集中的空间分配，实现高效的纹理空间利用率

use nalgebra::Point2;
use pi_hash::XHashMap;

/// 纹理装箱管理器
/// 
/// 负责管理纹理空间的分配和行布局优化
#[derive(Debug)]
pub struct TextPacker {
    /// 纹理图集的总宽度（像素）
    pub width: usize,
    /// 纹理图集的总高度（像素）
    pub height: usize,
    /// 当前已分配的最后垂直位置
    pub last_v: usize,
    /// 行高度到行信息的映射表
    /// - Key: 行高度
    /// - Value: (起始坐标, 已分配字符数)
    line_map: XHashMap<usize, (Point2<usize>, usize)>,
}

impl TextPacker {
    /// 清空所有分配记录，重置状态
    pub fn clear(&mut self) {
        self.line_map.clear();
        self.last_v = 0;
    }

    /// 创建新的纹理装箱管理器
    /// 
    /// # 参数
    /// - `width`: 纹理图集宽度
    /// - `height`: 纹理图集高度
    pub fn new(width: usize, height: usize) -> Self {
        TextPacker {
            width,
            height,
            line_map: XHashMap::default(),
            last_v: 0,
        }
    }
    /// 分配指定高度的新行
    /// 
    /// # 参数
    /// - `line_height`: 请求的行高度
    /// 
    /// # 返回值
    /// 返回TexLine实例用于具体字符分配
    /// 
    /// # 注意
    /// 实际分配高度会进行对齐优化：
    /// - 小于等于42像素的行按42像素对齐
    /// - 大于42像素的行按32像素步进对齐
    pub fn alloc_line(&mut self, mut line_height: usize) -> TexLine {
        //每行可容纳四种字号，提高利用率
        line_height = if line_height <= 42 {
            42
        } else {
            (line_height - 42) / 32 * 32 + 32 + 42
        };

        let v = self.last_v;
        let mut is_new = false;
        let line = self.line_map.entry(line_height).or_insert_with(|| {
            is_new = true;
            (Point2::new(0, v), 0)
        });
        // 如果是新分配的行， self.last_v + line_height
        if is_new {
            // self.last_v += line_height as f32 + 1.0; // 行与行之间间隔两个个像素，以免过界采样，出现细线；如果纹理不够时，先清空纹理为蓝色，重新更新纹理，则不会出现这个问题，因为文字周围本身就有空白
            self.last_v += line_height;
        }
        TexLine {
            line: line,
            last_v: &mut self.last_v,
            tex_width: self.width,
            line_height: line_height,
        }
    }

    /// 分配指定尺寸的字符空间
    /// 
    /// # 参数
    /// - `width`: 字符宽度
    /// - `height`: 字符高度
    /// 
    /// # 返回值
    /// 返回Option包装的起始坐标，None表示分配失败（超出纹理范围）
    pub fn alloc(&mut self, width: usize, height: usize) -> Option<Point2<usize>> {
        let mut line = self.alloc_line(height);
        let p = line.alloc(width);

        // 超出最大纹理范围，需要清空所有文字，重新布局
        if *(line.last_v) > self.height {
            return None; // 0表示异常情况，不能计算字形
        } else {
            Some(p)
        }
    }

    // fn update(&self, tex: Res<TextureRes>, u: f32, v: f32, w: f32, h: f32, data: &Object) {
    //     if v + h > self.last_v {
    //         // 纹理高度扩展1倍
    //     }
    //     self.tex.bind.update_webgl(tex, u, v, w, h, data);
    // }
}

/// 单行分配管理器
/// 
/// 负责单行内的水平空间分配和行状态管理
#[derive(Debug)]
pub struct TexLine<'a> {
    line: &'a mut (Point2<usize>, usize),
    pub last_v: &'a mut usize,
    pub tex_width: usize,
    line_height: usize,
}
impl<'a> TexLine<'a> {
    /// 获取当前行的垂直起始位置
    pub fn get_v(&self) -> usize {
        self.line.0.y
    }
    /// 分配字符的水平空间
    /// 
    /// # 参数
    /// - `char_width`: 字符宽度
    /// 
    /// # 返回值
    /// 返回字符的起始坐标
    /// 
    /// # 注意
    /// 当行空间不足时会自动换行并更新全局垂直位置
    pub fn alloc(&mut self, char_width: usize) -> Point2<usize> {
        if self.tex_width >= self.line.0.x + char_width {
            let r = self.line.0.clone();
            self.line.0.x += char_width;
            self.line.1 += 1;
            r
        } else {
            self.line.0.x = char_width;
            self.line.0.y = *self.last_v;
            self.line.1 = 1;
            *self.last_v += self.line_height;
            Point2::new(0, self.line.0.y)
        }
    }
}
