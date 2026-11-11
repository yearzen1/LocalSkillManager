//! skill 分组：组名、成员（skill 或组引用）与展开（Q60）。

use std::collections::HashSet;
use std::fmt;

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

/// 分组的一个成员：直接 skill，或引用另一个分组（`@name`）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Member {
    Skill(SkillName),
    Group(GroupName),
}

/// 成员解析失败；保留原始输入（含 `@`）给分组文案使用。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemberError {
    Skill(NameError),
    Group { raw: String, source: GroupNameError },
}

impl Member {
    /// `tdd` → skill，`@common` → 组引用。
    pub fn parse(input: &str) -> Result<Self, MemberError> {
        match input.strip_prefix('@') {
            Some(name) => {
                GroupName::parse(name)
                    .map(Member::Group)
                    .map_err(|source| MemberError::Group {
                        raw: input.to_string(),
                        source,
                    })
            }
            None => SkillName::parse(input)
                .map(Member::Skill)
                .map_err(MemberError::Skill),
        }
    }

    /// skill 成员。
    pub fn skill(&self) -> Option<&SkillName> {
        match self {
            Member::Skill(name) => Some(name),
            Member::Group(_) => None,
        }
    }

    /// 组引用成员。
    pub fn group(&self) -> Option<&GroupName> {
        match self {
            Member::Skill(_) => None,
            Member::Group(name) => Some(name),
        }
    }
}

impl fmt::Display for Member {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Member::Skill(name) => formatter.write_str(name.as_str()),
            Member::Group(name) => write!(formatter, "@{}", name.as_str()),
        }
    }
}

/// 一个分组定义；成员保持文件里的定义顺序，重复项已去重。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub name: GroupName,
    pub members: Vec<Member>,
}

/// 展开时的结构性错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpandError {
    /// 顶层分组不存在。
    MissingGroup { name: GroupName },
    /// 组内引用了不存在的分组。
    MissingReference {
        owner: GroupName,
        reference: GroupName,
    },
    /// 引用成环；`path` 形如 `a, b, a`。
    Cycle { path: Vec<GroupName> },
}

/// 深度优先展开；`@ref` 在原位置插入，skill 按首次出现去重。
pub fn expand(groups: &[Group], name: &GroupName) -> Result<Vec<SkillName>, ExpandError> {
    let mut skills = Vec::new();
    let mut seen = HashSet::new();
    let mut stack = Vec::new();
    visit(groups, name, &mut stack, &mut skills, &mut seen)?;
    Ok(skills)
}

fn visit(
    groups: &[Group],
    name: &GroupName,
    stack: &mut Vec<GroupName>,
    skills: &mut Vec<SkillName>,
    seen: &mut HashSet<SkillName>,
) -> Result<(), ExpandError> {
    if let Some(position) = stack.iter().position(|group| group == name) {
        let mut path = stack[position..].to_vec();
        path.push(name.clone());
        return Err(ExpandError::Cycle { path });
    }

    let group = groups
        .iter()
        .find(|group| group.name == *name)
        .ok_or_else(|| match stack.last() {
            Some(owner) => ExpandError::MissingReference {
                owner: owner.clone(),
                reference: name.clone(),
            },
            None => ExpandError::MissingGroup { name: name.clone() },
        })?;

    stack.push(name.clone());
    for member in &group.members {
        match member {
            Member::Skill(skill) => {
                if seen.insert(skill.clone()) {
                    skills.push(skill.clone());
                }
            }
            Member::Group(reference) => visit(groups, reference, stack, skills, seen)?,
        }
    }
    stack.pop();

    Ok(())
}

/// `start` 是否等于 `target`，或沿组引用能到达 `target`；用于写入时拒绝成环。
pub fn depends_on(groups: &[Group], start: &GroupName, target: &GroupName) -> bool {
    find_path(groups, start, target).is_some()
}

/// 从 `start` 沿组引用到 `target` 的路径（含两端）；用于成环错误文案。
pub fn find_path(
    groups: &[Group],
    start: &GroupName,
    target: &GroupName,
) -> Option<Vec<GroupName>> {
    fn walk(
        groups: &[Group],
        current: &GroupName,
        target: &GroupName,
        path: &mut Vec<GroupName>,
        visited: &mut HashSet<GroupName>,
    ) -> bool {
        path.push(current.clone());
        if current == target {
            return true;
        }
        if !visited.insert(current.clone()) {
            path.pop();
            return false;
        }

        if let Some(group) = groups.iter().find(|group| group.name == *current) {
            for member in &group.members {
                if let Member::Group(reference) = member {
                    if walk(groups, reference, target, path, visited) {
                        return true;
                    }
                }
            }
        }

        path.pop();
        false
    }

    let mut path = Vec::new();
    let mut visited = HashSet::new();
    walk(groups, start, target, &mut path, &mut visited).then_some(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn group_name(input: &str) -> GroupName {
        GroupName::parse(input).expect("test group name should be valid")
    }

    fn skill_name(input: &str) -> SkillName {
        SkillName::parse(input).expect("test skill name should be valid")
    }

    fn skill(input: &str) -> Member {
        Member::Skill(skill_name(input))
    }

    fn group(input: &str) -> Member {
        Member::Group(group_name(input))
    }

    fn definition(name: &str, members: Vec<Member>) -> Group {
        Group {
            name: group_name(name),
            members,
        }
    }

    #[test]
    fn group_names_use_skill_name_rules() {
        assert_eq!(GroupName::parse("frontend").unwrap().as_str(), "frontend");

        for input in ["Frontend", "a--b", "con", ""] {
            assert!(GroupName::parse(input).is_err(), "{input:?}");
        }
    }

    #[test]
    fn members_parse_skills_and_group_references() {
        assert_eq!(Member::parse("tdd"), Ok(skill("tdd")));
        assert_eq!(Member::parse("@common"), Ok(group("common")));
        assert!(Member::parse("@").is_err());
        assert!(Member::parse("@Bad").is_err());
        assert!(Member::parse("Bad").is_err());
    }

    #[test]
    fn members_render_with_the_reference_marker() {
        assert_eq!(skill("tdd").to_string(), "tdd");
        assert_eq!(group("common").to_string(), "@common");
    }

    #[test]
    fn expansion_is_depth_first_and_deduplicates() {
        let groups = [
            definition("common", vec![skill("tdd"), skill("grill")]),
            definition(
                "frontend",
                vec![group("common"), skill("tdd"), skill("docs-writer")],
            ),
        ];

        assert_eq!(
            expand(&groups, &group_name("frontend")).unwrap(),
            vec![
                skill_name("tdd"),
                skill_name("grill"),
                skill_name("docs-writer")
            ]
        );
    }

    #[test]
    fn expansion_reports_missing_groups_and_references() {
        let groups = [definition("frontend", vec![group("common")])];

        assert_eq!(
            expand(&groups, &group_name("nope")),
            Err(ExpandError::MissingGroup {
                name: group_name("nope")
            })
        );
        assert_eq!(
            expand(&groups, &group_name("frontend")),
            Err(ExpandError::MissingReference {
                owner: group_name("frontend"),
                reference: group_name("common"),
            })
        );
    }

    #[test]
    fn expansion_reports_the_cycle_path() {
        let groups = [
            definition("a", vec![group("b")]),
            definition("b", vec![group("a")]),
        ];

        assert_eq!(
            expand(&groups, &group_name("a")),
            Err(ExpandError::Cycle {
                path: vec![group_name("a"), group_name("b"), group_name("a")]
            })
        );
    }

    #[test]
    fn depends_on_walks_transitive_references() {
        let groups = [
            definition("a", vec![group("b")]),
            definition("b", vec![group("c")]),
            definition("c", vec![skill("tdd")]),
        ];

        assert!(depends_on(&groups, &group_name("a"), &group_name("c")));
        assert!(depends_on(&groups, &group_name("a"), &group_name("a")));
        assert!(!depends_on(&groups, &group_name("c"), &group_name("a")));
    }
}
