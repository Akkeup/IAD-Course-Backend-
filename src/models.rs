use serde::{
    Deserialize, Serialize
};
use std::{
    env, f32, format
};

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum StarStatus {
    Draft,
    Published,
    Deleted
}

#[derive(Debug, Serialize, Clone)]
pub struct Star {
    pub id: u32,
    pub name: String,
    pub catalog_id: String,
    pub distance_kpc: f32,
    pub status: StarStatus,
    pub description: String,
    pub image_url: String,
    pub video_url: String,
    pub likes: Vec<u32>
}

const ANDROMEDA_ROTATION_CURVE: &[(f32, u16)] = &[
    (0.0, 0),
    (2.0, 140),
    (4.0, 190),
    (6.3, 218),
    (8.7, 220),
    (18.4, 225),
    (22.6, 220),
    (25.0, 220),
    (30.0, 205),
    (33.5, 185),
];

pub fn calculate_velocity_kms(distance_kpc: f32) -> u16 {
    let distance_kpc = distance_kpc.max(0.0);

    for curve_segment in ANDROMEDA_ROTATION_CURVE.windows(2) {
        let (left_distance, left_velocity) = curve_segment[0];
        let (right_distance, right_velocity) = curve_segment[1];

        if distance_kpc <= right_distance {
            let distance_fraction =
                (distance_kpc - left_distance) / (right_distance - left_distance);
            let velocity = left_velocity as f32
                + distance_fraction * (right_velocity as f32 - left_velocity as f32);

            return velocity.round() as u16;
        }
    }

    ANDROMEDA_ROTATION_CURVE
        .last()
        .map(|(_, velocity)| *velocity)
        .unwrap_or(0)
}

impl Star {
    pub fn likes_count(&self) -> usize {
        return self.likes.len();
    }

    pub fn is_visible(&self) -> bool {
        return self.status != StarStatus::Deleted;
    }
}

#[derive(Debug)]
pub struct AppState {
    pub stars: Vec<Star>
}

impl AppState {
    pub fn from_env() -> Self {
        let minio_public_url = env::var("MINIO_PUBLIC_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:9000".to_owned());
        let minio_bucket = env::var("MINIO_BUCKET")
            .unwrap_or_else(|_| "andromeda".to_owned());

        Self::new(&minio_public_url, &minio_bucket)
    }

    pub fn new(minio_public_url: &str, minio_bucket: &str) -> Self {
        Self {
            stars: create_stars(minio_public_url, minio_bucket),
        }
    }
}

fn media_url(minio_public_url: &str, minio_bucket: &str, object_key: &str) -> String {
    format!(
        "{}/{}/{}",
        minio_public_url.trim_end_matches('/'),
        minio_bucket.trim_matches('/'),
        object_key.trim_start_matches('/')
    )
}

fn like_ids(count: u32) -> Vec<u32> {
    (1..=count).collect()
}

fn create_stars(minio_public_url: &str, minio_bucket: &str) -> Vec<Star> {
    return vec![
        Star {
            id: 1,
            name: "M31-V1".to_owned(),
            catalog_id: "M31-V1".to_owned(),
            distance_kpc: 18.4,
            status: StarStatus::Published,
            description: "Переменная звезда типа цефеиды, наблюдения которой помогли Эдвину Хабблу подтвердить, что Андромеда находится за пределами Млечного Пути. Скорость дана по учебной выборке точек кривой вращения M31.".to_owned(),
            image_url: media_url(minio_public_url, minio_bucket, "images/m31-v1.webp"),
            video_url: media_url(minio_public_url, minio_bucket, "videos/m31-v1.mp4"),
            likes: like_ids(135),
        },
        Star {
            id: 2,
            name: "M31-1775".to_owned(),
            catalog_id: "J004047.84+405602.6".to_owned(),
            distance_kpc: 18.4,
            status: StarStatus::Draft,
            description: "Красный сверхгигант с необычно сильным покраснением спектра. В каталоге LGGS объект обозначен координатным идентификатором J004047.84+405602.6.".to_owned(),
            image_url: media_url(minio_public_url, minio_bucket, "images/m31-1775.webp"),
            video_url: media_url(minio_public_url, minio_bucket, "videos/m31-1775.mp4"),
            likes: like_ids(22),
        },
        Star {
            id: 3,
            name: "M31-1515".to_owned(),
            catalog_id: "J004124.80+411634.7".to_owned(),
            distance_kpc: 8.7,
            status: StarStatus::Published,
            description: "Красный сверхгигант спектрального класса M3 I. Избыток излучения в ближнем ультрафиолете может указывать на горячий звёздный компонент.".to_owned(),
            image_url: media_url(minio_public_url, minio_bucket, "images/m31-1515.webp"),
            video_url: media_url(minio_public_url, minio_bucket, "videos/m31-1515.mp4"),
            likes: like_ids(56),
        },
        Star {
            id: 4,
            name: "M31-2252".to_owned(),
            catalog_id: "J004424.94+412322.3".to_owned(),
            distance_kpc: 22.6,
            status: StarStatus::Published,
            description: "Яркий красный сверхгигант в диске M31. Его положение соответствует почти плоской части кривой вращения галактики.".to_owned(),
            image_url: media_url(minio_public_url, minio_bucket, "images/m31-2252.webp"),
            video_url: media_url(minio_public_url, minio_bucket, "videos/m31-2252.mp4"),
            likes: like_ids(144),
        },
        Star {
            id: 5,
            name: "M31-1372".to_owned(),
            catalog_id: "J004454.38+412441.6".to_owned(),
            distance_kpc: 6.3,
            status: StarStatus::Published,
            description: "Красный сверхгигант спектрального класса M2 I из выборки звёзд Андромеды с подтверждённой лучевой скоростью.".to_owned(),
            image_url: media_url(minio_public_url, minio_bucket, "images/m31-1372.webp"),
            video_url: media_url(minio_public_url, minio_bucket, "videos/m31-1372.mp4"),
            likes: like_ids(89),
        },
        Star {
            id: 6,
            name: "M31-504".to_owned(),
            catalog_id: "J004447.08+412801.7".to_owned(),
            distance_kpc: 25.0,
            status: StarStatus::Published,
            description: "Красный сверхгигант спектрального класса M2.5 I у внешней границы почти плоской части кривой вращения M31.".to_owned(),
            image_url: media_url(minio_public_url, minio_bucket, "images/m31-504.webp"),
            video_url: media_url(minio_public_url, minio_bucket, "videos/m31-504.mp4"),
            likes: like_ids(41),
        },
        // Star {
        //     id: 7,
        //     name: "M31-1414".to_owned(),
        //     catalog_id: "J004501.30+413922.5".to_owned(),
        //     distance_kpc: 30.8,
        //     status: StarStatus::Published,
        //     description: "Красный сверхгигант спектрального класса M3 I. На таком расстоянии учебная аппроксимация показывает постепенное снижение скорости вращения.".to_owned(),
        //     image_url: media_url(minio_public_url, minio_bucket, "images/m31-1414.webp"),
        //     video_url: media_url(minio_public_url, minio_bucket, "videos/m31-1414.mp4"),
        //     likes: like_ids(73),
        // },
        Star {
            id: 8,
            name: "M31-1494".to_owned(),
            catalog_id: "J004514.95+414625.6".to_owned(),
            distance_kpc: 33.5,
            status: StarStatus::Deleted,
            description: "Удалённая карточка используется для проверки бизнес-правила и не должна попадать ни на одну страницу интерфейса.".to_owned(),
            image_url: media_url(minio_public_url, minio_bucket, "images/m31-1494.webp"),
            video_url: media_url(minio_public_url, minio_bucket, "videos/m31-1494.mp4"),
            likes: like_ids(12),
        },
    ];
}