#![feature(test)]
extern crate test;

use test::Bencher;
use via_router::Router;

static ROUTES: [&str; 100] = [
    "/home",
    "/about",
    "/contact",
    "/login",
    "/signup",
    "/profile/:user_name",
    "/user/:user_id",
    "/settings",
    "/settings/account",
    "/settings/privacy",
    "/settings/security",
    "/posts",
    "/post/:post_id",
    "/post/:post_id/edit",
    "/post/:post_id/comments",
    "/post/:post_id/comments/:comment_id",
    "/post/:post_id/likes",
    "/post/:post_id/share",
    "/comments",
    "/comment/:comment_id",
    "/notifications",
    "/notifications/:notification_id",
    "/messages",
    "/message/:message_id",
    "/message/:message_id/reply",
    "/search",
    "/search/results",
    "/search/:query",
    "/admin",
    "/admin/users",
    "/admin/user/:user_id",
    "/admin/user/:user_id/edit",
    "/admin/posts",
    "/admin/post/:post_id",
    "/admin/post/:post_id/edit",
    "/admin/comments",
    "/admin/comment/:comment_id",
    "/admin/comment/:comment_id/edit",
    "/admin/categories",
    "/admin/category/:category_id",
    "/admin/category/:category_id/edit",
    "/admin/tags",
    "/admin/tag/:tag_id",
    "/admin/tag/:tag_id/edit",
    "/admin/settings",
    "/categories",
    "/category/:category_id",
    "/category/:category_id/posts",
    "/tags",
    "/tag/:tag_id",
    "/tag/:tag_id/posts",
    "/favorites",
    "/favorite/:item_id",
    "/friends",
    "/friend/:friend_id",
    "/groups",
    "/group/:group_id",
    "/group/:group_id/members",
    "/group/:group_id/posts",
    "/events",
    "/event/:event_id",
    "/event/:event_id/rsvp",
    "/event/:event_id/attendees",
    "/help",
    "/help/article/:article_id",
    "/terms",
    "/privacy",
    "/faq",
    "/sitemap",
    "/rss",
    "/api/:version/:resource",
    "/api/:version/:resource/:resource_id",
    "/api/:version/:resource/:resource_id/edit",
    "/api/:version/:resource/:resource_id/comments/:comment_id",
    "/api/:version/:resource/:resource_id/comments/:comment_id/edit",
    "/checkout",
    "/checkout/cart",
    "/checkout/payment",
    "/checkout/confirmation",
    "/dashboard",
    "/dashboard/overview",
    "/dashboard/stats",
    "/dashboard/reports",
    "/notifications/settings",
    "/notifications/settings/email",
    "/notifications/settings/push",
    "/inbox",
    "/inbox/:conversation_id",
    "/inbox/:conversation_id/messages",
    "/subscriptions",
    "/subscription/:subscription_id",
    "/subscription/:subscription_id/edit",
    "/billing",
    "/billing/history",
    "/billing/payment-methods",
    "/billing/invoice/:invoice_id",
    "/report/user/:user_id",
    "/report/post/:post_id",
    "/report/comment/:comment_id",
    "/invite",
];

#[bench]
fn find_matches_1(b: &mut Bencher) {
    let mut router: Router<()> = Router::new();

    for path in ROUTES {
        let _ = router.at(path).get_or_insert_route_with(|| ());
    }

    b.iter(|| {
        for _ in router.visit("/dashboard") {
            // noop
        }
    });
}

#[bench]
fn find_matches_2(b: &mut Bencher) {
    let mut router: Router<()> = Router::new();

    for path in ROUTES {
        let _ = router.at(path).get_or_insert_route_with(|| ());
    }

    b.iter(|| {
        for _ in router.visit("/dashboard/overview") {
            // noop
        }
    });
}

#[bench]
fn find_matches_3(b: &mut Bencher) {
    let mut router: Router<()> = Router::new();

    for path in ROUTES {
        let _ = router.at(path).get_or_insert_route_with(|| ());
    }

    b.iter(|| {
        for _ in router.visit("/help/article/12345678987654321") {
            // noop
        }
    });
}

#[bench]
fn find_matches_4(b: &mut Bencher) {
    let mut router: Router<()> = Router::new();

    for path in ROUTES {
        let _ = router.at(path).get_or_insert_route_with(|| ());
    }

    b.iter(|| {
        for _ in router.visit("/api/v1/products/12345678987654321") {
            // noop
        }
    });
}

#[bench]
fn find_matches_5(b: &mut Bencher) {
    let mut router: Router<()> = Router::new();

    for path in ROUTES {
        let _ = router.at(path).get_or_insert_route_with(|| ());
    }

    b.iter(|| {
        for _ in router.visit("/api/v1/products/12345678987654321/comments/12345678987654321") {
            // noop
        }
    });
}

#[bench]
fn find_matches_6(b: &mut Bencher) {
    let mut router: Router<()> = Router::new();

    for path in ROUTES {
        let _ = router.at(path).get_or_insert_route_with(|| ());
    }

    b.iter(|| {
        for _ in router.visit("/api/v1/products/12345678987654321/comments/12345678987654321") {
            // noop
        }
    });
}

#[bench]
fn find_matches_7(b: &mut Bencher) {
    let mut router: Router<()> = Router::new();

    for path in ROUTES {
        let _ = router.at(path).get_or_insert_route_with(|| ());
    }

    b.iter(|| {
        for _ in router.visit("/api/v1/products/12345678987654321/comments/12345678987654321/edit")
        {
            // noop
        }
    });
}
