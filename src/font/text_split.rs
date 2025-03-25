//! 文字劈分算法
use std::str::Chars;

use pi_ucd::Codepoint;

use super::font::GlyphId;

#[derive(Debug)]
// 劈分结果
pub enum SplitResult {
    Newline(isize),
    Whitespace(isize),
    Word(isize,char),      // 单字词
    WordStart(isize,char), // 单词开始, 连续的字母或数字(必须字符的type_id相同)组成的单词
    WordNext(isize,char),  // 单词字符继续
    WordEnd(isize),         // 单词字符结束
}

// 劈分字符迭代器
pub struct SplitChar<'a> {
	cur_index: usize,
    iter: Chars<'a>,
    word_split: bool,
    merge_whitespace: bool,
    last: Option<char>,
    type_id: usize, // 0表示单字词, 1表示ascii字母 2及以上代表字符的type_id, MAX表示数字
}

impl<'a> Iterator for SplitChar<'a> {
    type Item = SplitResult;
    fn next(&mut self) -> Option<Self::Item> {
        match self.last {
            Some(c) if self.type_id == 0 => {
                if c == '\n' {
                    self.last = self.iter.next();
					self.cur_index += 1;
                    Some(SplitResult::Newline((self.cur_index - 1) as isize))
                } else if c.is_whitespace() {
                    if self.merge_whitespace {
                        loop {
							self.cur_index += 1;
                            match self.iter.next() {
                                Some(cc) if cc.is_whitespace() => continue,
                                r => {
                                    self.last = r;
                                    break;
                                }
                            }
                        }
                    } else {
                        self.last = self.iter.next();
						self.cur_index += 1;
                    }
                    Some(SplitResult::Whitespace((self.cur_index - 1) as isize))
                } else if !self.word_split {
                    self.last = self.iter.next();
					self.cur_index += 1;
                    Some(SplitResult::Word((self.cur_index - 1) as isize,c))
                } else {
                    self.type_id = get_type_id(c, char::from(0));
                    if self.type_id == 0 {
                        self.last = self.iter.next();
						self.cur_index += 1;
                        Some(SplitResult::Word((self.cur_index - 1) as isize,c))
                    } else {
                        // 如果是单词开始，不读取下个字符，因为需要保留当前字符做是否为单词的判断
                        Some(SplitResult::WordStart(self.cur_index as isize,c))
                    }
                }
            }
            Some(old_c) => {
                self.last = self.iter.next();
				self.cur_index += 1;
                match self.last {
                    Some(c) => {
                        let id = get_type_id(c, old_c);
                        if id == self.type_id {
                            Some(SplitResult::WordNext(self.cur_index as isize,c))
                        } else {
                            self.type_id = 0;
                            Some(SplitResult::WordEnd(-1))
                        }
                    }
                    _ => Some(SplitResult::WordEnd(-1)),
                }
            }
            _ => None,
        }
    }
}

/// 数字或字母, 返回对应的类型
fn get_type_id(c: char, prev: char) -> usize {
    if c.is_ascii() {
        if c.is_ascii_alphabetic() {
            return 1;
        } else if c.is_ascii_digit() {
            return usize::max_value();
        } else if c == '/' || c == '.' || c == '%' {
            if prev.is_ascii_digit() {
                return usize::max_value();
            }
        } else if c == '\'' {
            if prev.is_ascii_alphabetic() {
                return 1;
            }
        }
    } else if c.is_alphabetic() && !c.is_cased() {
        return c.get_type_id();
    }
    0
}
/// 劈分字符串, 返回字符迭代器
pub fn split<'a>(s: &'a str, word_split: bool, merge_whitespace: bool) -> SplitChar<'a> {
    let mut i = s.chars();
    let last = i.next();
    SplitChar {
		cur_index: 0,
        iter: i,
        word_split: word_split,
        merge_whitespace: merge_whitespace,
        last: last,
        type_id: 0,
    }
}

#[derive(Debug)]
// 劈分结果
pub enum SplitResult2 {
    Newline(isize),
    Whitespace(isize),
    Word(isize,char,Option<GlyphId>),      // 单字词
    WordStart(isize,char,Option<GlyphId>), // 单词开始, 连续的字母或数字(必须字符的type_id相同)组成的单词
    WordNext(isize,char,Option<GlyphId>),  // 单词字符继续
    WordEnd(isize),         // 单词字符结束
}

// 劈分字符迭代器
pub struct SplitChar2<'a> {
    pub(crate) cur_index: usize,
    pub(crate) iter: Chars<'a>,
    pub(crate) word_split: bool,
    pub(crate) merge_whitespace: bool,
    pub(crate) last: Option<char>,
    pub(crate) type_id: usize, // 0表示单字词, 1表示ascii字母 2及以上代表字符的type_id, MAX表示数字
    pub(crate) glyph_ids: Vec<Option<GlyphId>>, // 每个字符的字形id
}

impl<'a> Iterator for SplitChar2<'a> {
    type Item = SplitResult2;
    fn next(&mut self) -> Option<Self::Item> {
        match self.last {
            Some(c) if self.type_id == 0 => {
                if c == '\n' {
                    self.last = self.iter.next();
					self.cur_index += 1;
                    Some(SplitResult2::Newline((self.cur_index - 1) as isize))
                } else if c.is_whitespace() {
                    if self.merge_whitespace {
                        loop {
							self.cur_index += 1;
                            match self.iter.next() {
                                Some(cc) if cc.is_whitespace() => continue,
                                r => {
                                    self.last = r;
                                    break;
                                }
                            }
                        }
                    } else {
                        self.last = self.iter.next();
						self.cur_index += 1;
                    }
                    Some(SplitResult2::Whitespace((self.cur_index - 1) as isize))
                } else if !self.word_split {
                    self.last = self.iter.next();
					self.cur_index += 1;
                    let id =  self.glyph_ids.get(self.cur_index - 1);
                    let id = if let Some(v) = id{*v}else{None};
                    Some(SplitResult2::Word((self.cur_index - 1) as isize,c,id))
                } else {
                    self.type_id = get_type_id(c, char::from(0));
                    if self.type_id == 0 {
                        self.last = self.iter.next();
						self.cur_index += 1;
                        let id =  self.glyph_ids.get(self.cur_index - 1);
                        let id = if let Some(v) = id{*v}else{None};
                        Some(SplitResult2::Word((self.cur_index - 1) as isize,c, id))
                    } else {
                        // 如果是单词开始，不读取下个字符，因为需要保留当前字符做是否为单词的判断
                        let id =  self.glyph_ids.get(self.cur_index);
                        let id = if let Some(v) = id{*v}else{None};
                        Some(SplitResult2::WordStart(self.cur_index as isize,c,id))
                    }
                }
            }
            Some(old_c) => {
                self.last = self.iter.next();
				self.cur_index += 1;
                match self.last {
                    Some(c) => {
                        let id = get_type_id(c, old_c);
                        if id == self.type_id {
                            let id =  self.glyph_ids.get(self.cur_index);
                            let id = if let Some(v) = id{v.clone()}else{None};
                            Some(SplitResult2::WordNext(self.cur_index as isize,c,id))
                        } else {
                            self.type_id = 0;
                            Some(SplitResult2::WordEnd(-1))
                        }
                    }
                    _ => Some(SplitResult2::WordEnd(-1)),
                }
            }
            _ => None,
        }
    }
}