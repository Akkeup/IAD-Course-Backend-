# Andromeda Stars Backend

Учебный backend каталога звёзд галактики Андромеды для лабораторных работ 2 и 3.

Стек: Rust, Actix Web, Tera, SeaORM, SQLx, PostgreSQL, MinIO и Docker Compose.

## Запуск

1. Создать локальный файл окружения:

```sh
cp .env.example .env
```

2. Запустить инфраструктуру:

```sh
docker compose up -d
```

3. Запустить backend:

```sh
cargo run --locked
```

После запуска доступны:

- приложение: `http://127.0.0.1:8080/andromeda-stars/first`;
- Adminer: `http://127.0.0.1:8081`;
- MinIO API: `http://127.0.0.1:9000`;
- MinIO Console: `http://127.0.0.1:9001`.

Данные для Adminer: система `PostgreSQL`, сервер `postgres`, пользователь
`andromeda`, пароль `andromeda_password`, база `andromeda`.

SQLx автоматически применяет файлы из `migrations/` при запуске backend.

## База данных

Каскадное удаление не используется. Внешние ключи работают с поведением
PostgreSQL по умолчанию `NO ACTION`.

### `andromeda_users`

- `user_id BIGSERIAL` — первичный ключ;
- `username VARCHAR(100)` — обязательное уникальное имя;
- `password_hash TEXT` — хеш пароля;
- `created_at TIMESTAMPTZ` — дата регистрации.

### `andromeda_stars`

- `star_id BIGSERIAL` — первичный ключ;
- `created_by BIGINT` — внешний ключ на `andromeda_users.user_id`;
- `star_name VARCHAR(150)` — название звезды;
- `star_catalog_id VARCHAR(150)` — уникальный каталожный идентификатор;
- `distance_kpc NUMERIC(6,2)` — расстояние от центра галактики;
- `velocity_kms INTEGER` — скорость;
- `star_status VARCHAR(20)` — `draft`, `published` или `deleted`;
- `star_description TEXT` — краткая информация;
- `image_url TEXT` — имя изображения в MinIO или внешний URL;
- `video_url TEXT` — имя видео в MinIO или внешний URL;
- `created_at TIMESTAMPTZ` — дата создания;
- `updated_at TIMESTAMPTZ` — дата последнего изменения;
- `formed_at TIMESTAMPTZ NULL` — дата публикации.

Частичный уникальный индекс `one_draft_per_user` гарантирует не более одного
черновика у пользователя. Ограничение `published_star_formed_at` запрещает
опубликованную карточку без даты формирования.

### `andromeda_likes`

- `user_id BIGINT` — внешний ключ на пользователя;
- `star_id BIGINT` — внешний ключ на звезду;
- `created_at TIMESTAMPTZ` — дата лайка;
- составной первичный ключ `(user_id, star_id)` запрещает повторный лайк.

Таблица реализует связь многие-ко-многим между пользователями и звёздами.

## Лабораторная 2: SSR

Три страницы рендерятся Tera-шаблонами на сервере:

- лента: `GET /andromeda-stars/{id}`;
- добавление и публикация: `GET /andromeda-stars/draft`;
- плитка и поиск: `GET /andromeda-stars/grid`.

Всего используется шесть предметных обработчиков:

- `GET /andromeda-stars/{id}` — получает одну опубликованную строку;
- `GET /andromeda-stars/draft` — получает один черновик пользователя;
- `GET /andromeda-stars/grid` — фильтрует опубликованные записи в PostgreSQL;
- `POST /andromeda-stars/draft` — создаёт черновик через SeaORM;
- `POST /andromeda-stars/{id}/publish` — публикует через SeaORM;
- `POST /andromeda-stars/{id}/delete` — выполняет ручной SQL `UPDATE` через SQLx.

Удаление логическое: значение `star_status` меняется на `deleted`. Физическая
строка остаётся в таблице. Удалённая карточка не открывается, потому что методы
чтения запрашивают только `published`.

Во второй лабораторной выбранные файлы не отправляются: у HTML-input нет
атрибута `name`. Пустые или недоступные ссылки заменяются в HTML файлами из
`static/defaults/` с помощью `onerror`.

## Лабораторная 3: REST API

Все методы имеют префикс `/api`. Текущий пользователь лабораторной зафиксирован
константой `CURRENT_USERNAME` в `src/database.rs` и получается через singleton.

### Услуги

- `GET /api/andromeda-stars?distance_kpc=20&page=1`
  возвращает опубликованный список с фильтрацией и признаком `is_owner`.
- `GET /api/andromeda-stars/feed`
  возвращает первую опубликованную карточку.
- `GET /api/andromeda-stars/feed?id=5&next=true`
  возвращает следующую опубликованную карточку.
- `GET /api/andromeda-stars/draft`
  возвращает единственный черновик текущего пользователя.
- `POST /api/andromeda-stars`
  принимает `multipart/form-data`: `name`, `image`, `video`; ответ `201`.
- `PUT /api/andromeda-stars/{id}/publish`
  принимает JSON с `name`, `catalog_id`, `distance_kpc`, `velocity_kms`,
  `description`; ответ `204`.
- `DELETE /api/andromeda-stars/{id}`
  логически удаляет только собственную активную карточку; ответ `204`.
- `POST /api/andromeda-stars/{id}/like`
  принимает JSON `{"value":1}` или `{"value":0}`; ответ `204`.

### Пользователи

- `POST /api/users/register` принимает `username` и `password`; ответ `201`.
- `POST /api/auth/login` — заглушка лабораторной 4; ответ `204`.
- `POST /api/auth/logout` — заглушка лабораторной 4; ответ `204`.

Изменяющие методы и ошибки возвращают только HTTP-код с пустым телом. JSON
возвращают только GET-методы, которым необходимо передать предметные данные.

## MinIO

При создании карточки API принимает реальные файлы, а не URL. Изображение и
видео сохраняются в bucket `andromeda`. В PostgreSQL записываются имена вида:

```text
images/550e8400-e29b-41d4-a716-446655440000.png
videos/550e8400-e29b-41d4-a716-446655440001.mp4
```

UUID содержит только латинские шестнадцатеричные символы, цифры и дефисы.
Поддерживаются JPEG, PNG, WebP и MP4. Сервер проверяет лимит multipart,
заявленный MIME и сигнатуру содержимого. Если загрузка видео или запись в БД
завершается ошибкой, уже загруженные объекты удаляются.

`ANDROMEDA_MINIO_ENDPOINT` задаёт внутренний адрес S3 API, а
`ANDROMEDA_MINIO_PUBLIC_URL` используется в ссылках, возвращаемых клиенту.

## Безопасность

- SQL формируется SeaORM или параметризованным SQLx, пользовательские значения
  не объединяются со строкой запроса;
- DTO с `deny_unknown_fields` отклоняют попытки передать статус, владельца, ID
  или системные даты;
- публикация и удаление проверяют `created_by` и допустимый исходный статус;
- удалённые записи не выдаются клиенту;
- пароль регистрации хешируется Argon2 с индивидуальной случайной солью;
- ответы с ошибками не раскрывают сообщения PostgreSQL и внутренние пути;
- ограничения БД дублируют критические правила приложения.

Фиксированный пользователь и заглушки авторизации допустимы только в рамках
лабораторной 3. Для реального сервиса требуются сессия или токен, CSRF-защита
для cookie-аутентификации, отдельный пользователь MinIO и HTTPS.

## Проверка

```sh
cargo fmt --check
cargo check --locked --offline
cargo test --locked --offline
cargo clippy --locked --offline --all-targets -- -A clippy::needless_return -D warnings
```

Полное объяснение реализации находится в
`docs/lab2-lab3-explanation.md`, ответы для защиты — в
`docs/control-questions-lab2-lab3.md`.
