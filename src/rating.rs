use crate::db::User;
use log::warn;
use scraper::{Html, Selector, ElementRef};

pub(crate) struct Rating {
    pub(crate) user: User,
    pub(crate) subjects: Vec<Subject>
}

#[derive(Debug, sqlx::FromRow)]
pub(crate) struct Subject {
    pub(crate) name: String,
    pub(crate) attendance: f32,
    pub(crate) control: f32,
    pub(crate) creative: f32,
    pub(crate) test: f32
}

impl Subject {
    pub fn to_string(&self) -> String {
        format!("{}:\nПосещаемость: {}\nТворческий: {}\nКонтрольный: {}\nЭкз/зачет: {}\nВсего: {}", 
            &self.name, 
            &self.attendance, 
            &self.creative,
            &self.control, 
            &self.test,
            &self.attendance + &self.control + &self.creative + &self.test
        )
    }
}

fn parse_rating_value(subject_elem: &ElementRef, item_selector: &Selector, value_selector: &Selector) -> Option<f32> {
    let item_elem: Vec<ElementRef> = subject_elem.select(&item_selector).collect();
    if item_elem.len() != 1 { warn!("Couldn't find value of the subject"); return None; }

    let item_value: Vec<ElementRef> = item_elem[0].select(&value_selector).collect();
    if item_value.len() != 1 { warn!("Couldn't find value of the subject"); return None; }

    let value = item_value[0].inner_html().trim().parse::<f32>();
    if value.is_err() { warn!("Error parsing value value"); return None; } 
    Some(value.unwrap())
}

fn parse_test_value(subject_elem: &ElementRef, item_selector: &Selector) -> Option<f32> {
    let item_elem: Vec<ElementRef> = subject_elem.select(&item_selector).collect();
    if item_elem.len() != 1 { warn!("Couldn't find attandance of the subject"); return None; }

    let value = item_elem[0].inner_html().trim().parse::<f32>();
    if value.is_err() { warn!("Error parsing attendance value"); return None; } 
    Some(value.unwrap())
}

pub(crate) async fn get_rating(user: User) -> Option<Rating> {
    let params = [
        ("AUTH_FORM", "Y"), 
        ("TYPE", "AUTH"), 
        ("backurl", "/index.php"), 
        ("USER_LOGIN", &user.username),
        ("USER_PASSWORD", &user.pwd),
        ("Login", "Войти"),
        ("login", "yes"),
        ("semester", &format!("{}-й семестр", user.semester))
    ];

    let client = reqwest::ClientBuilder::new()
    .danger_accept_invalid_certs(true)
    .cookie_store(true)
    .build().unwrap();

    let title_selector = Selector::parse("title").unwrap();

    let auth_res = client.post("https://student.rea.ru/index.php")
    .form(&params[0..5])
    .query(&params[6..7])
    .send()
    .await;
    if auth_res.is_err() {
        warn!("Reqwest error while sending rea auth request ({})", auth_res.err().unwrap().to_string());
        return None;
    }

    let auth_res_text = auth_res.unwrap().text().await;
    if auth_res_text.is_err() {
        warn!("Couldn't get auth request text Err({})", auth_res_text.err().unwrap().to_string());
        return None;
    }

    {
        let auth_html = Html::parse_document(&auth_res_text.unwrap());
        let titles: Vec<ElementRef> = auth_html.select(&title_selector).collect();
        if titles.len() != 1 || titles[0].inner_html() != "Информация об обучающемся" {
            warn!("no auth");
            return None;
        }
    };

    let rating_res = client.get("https://student.rea.ru/rating/index.php")
    .query(&params[7..8])
    .send()
    .await;
    if rating_res.is_err() {
        warn!("Reqwest error while sending rea auth request ({})", rating_res.err().unwrap().to_string());
        return None;
    }
    let rating_res_text = rating_res.unwrap().text().await;
    if rating_res_text.is_err() {
        warn!("Couldn't get rating request text Err({})", rating_res_text.err().unwrap().to_string());
        return None;
    }

    let rating_html = Html::parse_document(&rating_res_text.unwrap());

    let subjects_selector = Selector::parse("div.es-rating__line-parent").unwrap();
    let name_selector = Selector::parse("div.es-rating__discipline").unwrap();

    let attendance_selector = Selector::parse("div.es-rating__attendance").unwrap();
    let control_selector = Selector::parse("div.es-rating__control").unwrap();
    let creative_selector = Selector::parse("div.es-rating__creative").unwrap();
    let test_selector = Selector::parse("div.es-rating__form").unwrap();
    
    let number_selector = Selector::parse("a").unwrap();

    let mut rating = Rating {user, subjects: vec![] };
    for subject_elem in rating_html.select(&subjects_selector) {
        let subject_name: Vec<ElementRef> = subject_elem.select(&name_selector).collect();
        if subject_name.len() != 1 { warn!("Couldn't find name of the subject"); return None; }

        rating.subjects.push(
            Subject { 
                name: subject_name[0].inner_html().trim().to_string(), 
                attendance: parse_rating_value(&subject_elem, &attendance_selector, &number_selector)?,
                control: parse_rating_value(&subject_elem, &control_selector, &number_selector)?,
                creative: parse_rating_value(&subject_elem, &creative_selector, &number_selector)?,
                test: parse_test_value(&subject_elem, &test_selector)?
            }
        );
    }
    Some(rating)
}