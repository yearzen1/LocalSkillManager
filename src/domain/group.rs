//! skill 分组：组名规则与成员列表（Q53）。

use super::name::{NameError, SkillName};

/// 已校验的分组名；规则与 skill 名完全相同。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GroupName(SkillName);

/// 分组名校验失败；包一层只为让 CLI 用分组专属文案。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupNameError(pub NameError);

impl GroupName {
    /// 解析并完成全部校验（格式、长度、Windows 保留名）。
    pub fn parse(input: &str) -> Result<Self, GroupNameError> {
        SkillName::parse(input).map(Self).map_err(GroupNameError)
    }

    /// 已校验的原始值。
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// 一个分组定义；成员保持文件里的定义顺序，重复项已去重。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub name: GroupName,
    pub members: Vec<SkillName>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_names_use_skill_name_rules() {
        assert_eq!(GroupName::parse("frontend").unwrap().as_str(), "frontend");

        for input in ["Frontend", "a--b", "con", ""] {
            assert!(GroupName::parse(input).is_err(), "{input:?}");
        }
    }
}
