//! 文字装箱算法（货架算法）

use nalgebra::Point2;
use pi_hash::XHashMap;

pub struct TextPacker{
	pub width: usize,
	pub height: usize,
	pub last_v: usize,
    line_map: XHashMap<usize, (Point2<usize>, usize)>,
}

impl TextPacker {
	pub fn clear(&mut self) {
		self.line_map.clear();
		self.last_v = 0;
	}

    pub fn new(width: usize, height: usize) -> Self {
        TextPacker {
            width,
			height,
            line_map: XHashMap::default(),
            last_v: 0,
        }
    }
    // 分配行
    pub fn alloc_line(&mut self, mut line_height: usize) -> TexLine {
        // 将奇数的行高向上变成偶数，这样单行容纳2种字号，提高利用率
        if line_height %2 != 0 {
            line_height += 1;
        }
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
            line_height: line_height
        }
    }

	pub fn alloc(&mut self, width: usize, height: usize) -> Option<Point2<usize>> {
		let mut line = self.alloc_line(height);
		let p = line.alloc(width);

		// 超出最大纹理范围，需要清空所有文字，重新布局
		if *(line.last_v) > line.tex_width {
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

#[derive(Debug)]
pub struct TexLine<'a> {
    line: &'a mut (Point2<usize>, usize),
    pub last_v: &'a mut usize,
    pub tex_width: usize,
    line_height: usize,
}
impl<'a> TexLine<'a> {
    // 获得起始v
    pub fn get_v(&self) -> usize {
        self.line.0.y
    }
    // 分配字符的起始uv
    pub fn alloc(&mut self, char_width: usize) -> Point2<usize> {
        if self.tex_width >= self.line.0.x + char_width {
            let r = self.line.0.clone();
            self.line.0.x += char_width;
            self.line.1 += 1;
            r
        }else{
            self.line.0.x = char_width;
            self.line.0.y = *self.last_v;
            self.line.1 = 1;
            *self.last_v += self.line_height;
            Point2::new(0, self.line.0.y)
        }
    }
}
