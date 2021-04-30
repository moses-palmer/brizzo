const BASE_URL = (window.location + "")
    .replace(/[^/]*$/, "") + "../api/";


function create(name, text, shape) {
    let init = {
        method: "POST",
        headers: {
            "Content-Type": "application/json",
        },
        body: JSON.stringify({
            name: name,
            text: text,
            shape: shape,
            seed: 12345,
        }),
    };
    fetch(BASE_URL, init)
        .then((r) => {
            if (!r.ok) {
                throw r;
            } else {
                return r.blob().then(_ => r);
            }
        })
        .then(() => {
            window.location.href = "../show/#" + name;
        })
        .catch((r) => {
            switch (r.status) {
                case 409:
                    alert(""
                        + "A message with that name already exists. Please "
                        + "select a different name.");
                    return;
            }
            r.text().then(text => alert("Failed to create message: " + text));
        });
}
