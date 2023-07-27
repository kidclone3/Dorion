use tauri::regex::Regex;

#[tauri::command]
pub async fn localize_imports(win: tauri::Window, css: String) -> String {
  let reg = Regex::new(r#"(?m)^@import url\((?:"|'|)(http.*?\.css)(?:"|'|)\);"#).unwrap();
  let url_reg = Regex::new(r"\((.*)\)").unwrap();
  let mut seen_urls: Vec<String> = vec![];
  let mut new_css = css.clone();

  while reg.is_match(new_css.clone().as_str()) {
    let first_match = reg.find_iter(&new_css).next().unwrap();
    let url = url_reg
      .captures(first_match.as_str())
      .unwrap()
      .get(1)
      .unwrap()
      .as_str()
      // Remove quotes
      .replace(['\'', '\"'], "");

    if url.is_empty() {
      continue;
    }

    if seen_urls.contains(&url) {
      // Remove the import statement from the css
      new_css = new_css.replace(first_match.as_str(), "");
      continue;
    }

    println!("Getting: {}", &url);

    let response = match reqwest::get(&url).await {
      Ok(r) => r,
      Err(e) => {
        println!("Request failed: {}", e);
        println!("URL: {}", &url);

        new_css = new_css.replace(first_match.as_str(), "");
        continue;
      }
    };

    let status = response.status();

    if status != 200 {
      println!("Request failed: {}", status);
      println!("URL: {}", &url);

      new_css = new_css.replace(first_match.as_str(), "");
      continue;
    }

    let text = response.text().await.unwrap();

    // Emit a loading log
    win.emit("loading_log", format!("Processed CSS import: {}", url.clone())).unwrap();

    seen_urls.push(url.clone());

    new_css = new_css.replace(first_match.as_str(), text.as_str());
  }

  win.emit("loading_log", format!("Finished processing {} CSS imports", seen_urls.len())).unwrap();

  // Now localize images to base64 data representations
  new_css = localize_images(win, new_css).await;

  new_css
}

pub async fn localize_images(win: tauri::Window, css: String) -> String {
  let img_reg = Regex::new(r#"url\((?:"|'|)(http.*?\.png|.jpeg|.jpg|.gif)(?:"|'|)\);"#).unwrap();
  let matches = img_reg.captures_iter(&css);
  let mut new_css = css.clone();

  for groups in matches {
    let url = groups.get(1).unwrap().as_str();
    let filetype = url.split('.').last().unwrap();

    if url.is_empty() {
      continue;
    }

    let response = match reqwest::get(url).await {
      Ok(r) => r,
      Err(e) => {
        println!("Request failed: {}", e);
        println!("URL: {}", &url);

        win.emit("loading_log", format!("An image failed to import...")).unwrap();

        continue;
      }
    };
    let bytes = response.bytes().await.unwrap();
    let b64 = base64::encode(bytes);
    
    win.emit("loading_log", format!("Processed image import: {}", &url)).unwrap();

    new_css = new_css.replace(
      url,
      format!("data:image/{};base64,{}", filetype, b64).as_str(),
    )
  }

  new_css
}
