use crate::database;
use uuid::Uuid;

// 使用database模块中的QuickText结构
pub use crate::database::QuickText;

// 从数据库加载常用文本（数据库模式下不需要显式加载）
pub fn load_quick_texts() {
    println!("常用文本将从数据库动态加载");
}

// 获取所有常用文本
pub fn get_all_quick_texts() -> Vec<QuickText> {
    match database::get_all_quick_texts() {
        Ok(texts) => texts,
        Err(e) => {
            println!("从数据库获取常用文本失败: {}", e);
            Vec::new()
        }
    }
}

// 按分组获取常用文本
pub fn get_quick_texts_by_group(group_id: &str) -> Vec<QuickText> {
    match database::get_quick_texts_by_group(group_id) {
        Ok(texts) => texts,
        Err(e) => {
            println!("从数据库获取分组常用文本失败: {}", e);
            Vec::new()
        }
    }
}

// 添加常用文本到指定分组
pub fn add_quick_text_with_group(
    title: String,
    content: String,
    group_id: String,
) -> Result<QuickText, String> {
    add_quick_text_with_group_and_html(title, content, None, group_id)
}

// 添加常用文本到指定分组
pub fn add_quick_text_with_group_and_html(
    title: String,
    content: String,
    html_content: Option<String>,
    group_id: String,
) -> Result<QuickText, String> {
    let now_local = chrono::Local::now();
    let now = now_local.timestamp();
    let quick_text = QuickText {
        id: Uuid::new_v4().to_string(),
        title,
        content,
        html_content,
        created_at: now,
        updated_at: now,
        group_id,
    };

    // 保存到数据库
    database::add_quick_text(&quick_text)?;
    println!("常用文本已保存到数据库");

    Ok(quick_text)
}


// 更新常用文本（包含分组）
pub fn update_quick_text_with_group(
    id: String,
    title: String,
    content: String,
    group_id: Option<String>,
) -> Result<QuickText, String> {
    let now_local = chrono::Local::now();
    let now = now_local.timestamp();
    let group_id = group_id.unwrap_or_else(|| "all".to_string());

    let updated_text = QuickText {
        id: id.clone(),
        title,
        content,
        html_content: None,
        created_at: 0,
        updated_at: now,
        group_id,
    };

    // 在数据库中更新
    database::update_quick_text(&updated_text)?;
    println!("常用文本已在数据库中更新");

    Ok(updated_text)
}

// 删除常用文本
pub fn delete_quick_text(id: String) -> Result<(), String> {
    database::delete_quick_text(&id)?;
    println!("常用文本已从数据库中删除");
    Ok(())
}

// 移动单个常用文本到指定位置
pub fn move_item(item_id: String, to_index: usize) -> Result<(), String> {
    // 获取所有常用文本
    let all_texts =
        database::get_all_quick_texts().map_err(|e| format!("获取常用文本失败: {}", e))?;

    // 找到要移动的项目
    let item_index = all_texts
        .iter()
        .position(|t| t.id == item_id)
        .ok_or_else(|| format!("常用文本 {} 不存在", item_id))?;

    if to_index >= all_texts.len() {
        return Err(format!("目标索引 {} 超出范围", to_index));
    }

    if item_index == to_index {
        return Ok(()); // 没有移动，直接返回
    }

    // 获取要移动的项目的分组ID
    let item_group_id = &all_texts[item_index].group_id;

    // 获取同一分组的所有项目
    let mut group_texts: Vec<QuickText> = all_texts
        .iter()
        .filter(|t| &t.group_id == item_group_id)
        .cloned()
        .collect();

    // 找到在分组内的索引
    let group_item_index = group_texts
        .iter()
        .position(|t| t.id == item_id)
        .ok_or_else(|| format!("在分组中找不到常用文本 {}", item_id))?;

    if to_index >= group_texts.len() {
        return Err(format!("目标索引 {} 超出分组范围", to_index));
    }

    // 在分组内重新排列
    let moved_item = group_texts.remove(group_item_index);
    group_texts.insert(to_index, moved_item);

    // 调用数据库重新排序
    database::reorder_quick_texts(&group_texts)
        .map_err(|e| format!("数据库重新排序失败: {}", e))?;

    Ok(())
}

// 重新排序常用文本
pub fn reorder_quick_texts(items: Vec<QuickText>) -> Result<(), String> {
    // 验证传入的项目，确保它们都存在且 group_id 一致
    let mut group_id_check: Option<String> = None;
    for item in &items {
        if let Ok(exists) = database::quick_text_exists(&item.id) {
            if !exists {
                return Err(format!("常用文本 {} 不存在", item.id));
            }

            // 获取现有的分组ID进行验证
            if let Ok(existing_texts) = database::get_all_quick_texts() {
                if let Some(existing) = existing_texts.iter().find(|t| t.id == item.id) {
                    if group_id_check.is_none() {
                        group_id_check = Some(existing.group_id.clone());
                    } else if group_id_check.as_ref() != Some(&existing.group_id) {
                        return Err("不能跨分组排序常用文本".to_string());
                    }
                }
            }
        } else {
            return Err(format!("常用文本 {} 不存在", item.id));
        }
    }

    // 在数据库中重新排序
    database::reorder_quick_texts(&items)?;

    let group_name = group_id_check.unwrap_or_else(|| "未知".to_string());
    println!(
        "已在数据库中重新排序分组 {} 中的 {} 个常用文本",
        group_name,
        items.len()
    );

    Ok(())
}

// 移动常用文本到指定分组
pub fn move_quick_text_to_group(id: String, group_id: String) -> Result<(), String> {
    // 获取现有的常用文本
    let texts = database::get_all_quick_texts()?;
    let existing_text = texts
        .iter()
        .find(|t| t.id == id)
        .ok_or_else(|| format!("常用文本 {} 不存在", id))?;

    let old_group_id = existing_text.group_id.clone();

    // 创建更新后的文本
    let mut updated_text = existing_text.clone();
    updated_text.group_id = group_id.clone();
    updated_text.updated_at = chrono::Local::now().timestamp();

    // 在数据库中更新
    database::update_quick_text(&updated_text)?;

    println!(
        "已将常用文本 {} 从分组 {} 移动到分组 {}",
        id, old_group_id, group_id
    );
    Ok(())
}

// 将指定分组中的所有常用文本移动到"全部"分组
pub fn move_group_texts_to_all(group_id: &str) -> Result<(), String> {
    let texts = database::get_quick_texts_by_group(group_id)?;
    let mut moved_count = 0;

    for mut text in texts {
        text.group_id = "all".to_string();
        text.updated_at = chrono::Local::now().timestamp();

        if let Err(e) = database::update_quick_text(&text) {
            println!("移动常用文本 {} 失败: {}", text.id, e);
        } else {
            moved_count += 1;
        }
    }

    if moved_count > 0 {
        println!(
            "已将 {} 个常用文本从分组 {} 移动到全部分组",
            moved_count, group_id
        );
    } else {
        println!("分组 {} 中没有常用文本需要移动", group_id);
    }

    Ok(())
}

// 检查常用文本是否存在
pub fn quick_text_exists(id: &str) -> Result<bool, String> {
    database::quick_text_exists(id)
}

// 根据内容搜索常用文本
pub fn search_quick_texts(query: &str) -> Vec<QuickText> {
    match database::get_all_quick_texts() {
        Ok(texts) => texts
            .into_iter()
            .filter(|text| {
                text.title.to_lowercase().contains(&query.to_lowercase())
                    || text.content.to_lowercase().contains(&query.to_lowercase())
            })
            .collect(),
        Err(e) => {
            println!("搜索常用文本失败: {}", e);
            Vec::new()
        }
    }
}

// 获取分组中的常用文本数量
pub fn get_group_text_count(group_id: &str) -> usize {
    match database::get_quick_texts_by_group(group_id) {
        Ok(texts) => texts.len(),
        Err(_) => 0,
    }
}

// 获取常用文本的统计信息
pub fn get_quick_texts_stats() -> (usize, usize) {
    match database::get_all_quick_texts() {
        Ok(texts) => {
            let total_count = texts.len();
            let groups_count = texts
                .iter()
                .map(|t| &t.group_id)
                .collect::<std::collections::HashSet<_>>()
                .len();
            (total_count, groups_count)
        }
        Err(_) => (0, 0),
    }
}
