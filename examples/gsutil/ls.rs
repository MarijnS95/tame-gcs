use crate::util;
use ansi_term::Color;
use failure::Error;
use structopt::StructOpt;
use tame_gcs::{
    common::StandardQueryParameters,
    objects::{ListOptional, ListResponse, Metadata, Object},
};

#[derive(StructOpt, Debug)]
pub(crate) struct Args {
    /// Recurse into directories (not implemented!)
    #[structopt(short = "R", long)]
    recurse: bool,
    /// Displays extended metadata as a table
    #[structopt(short = "l", long)]
    long: bool,
    /// The gs:// url list out
    url: url::Url,
}

pub(crate) fn cmd(ctx: &util::RequestContext, args: Args) -> Result<(), Error> {
    let oid = util::gs_url_to_object_id(&args.url)?;

    let delimiter = if args.recurse { None } else { Some("/") };
    let mut prefix = oid.object().map(|on| on.as_ref()).unwrap_or("").to_owned();
    if !prefix.is_empty() && !prefix.ends_with('/') {
        prefix.push('/')
    }

    let prefix_len = prefix.len();
    let prefix = Some(prefix);

    let display = if args.long {
        Display::Long
    } else {
        Display::Normal
    };

    let mut normal = None;
    //let recurse = None;
    if args.recurse {
        // recurse = Some(RecursePrinter {
        //     display,
        //     prefix_len,
        //     dirs: HashSet::new(),
        // });
    } else {
        normal = Some(NormalPrinter {
            display,
            prefix_len,
        });
    }

    let fields = match display {
        Display::Normal => "items(name), prefixes, nextPageToken",
        Display::Long => "items(name, updated, size), prefixes, nextPageToken",
    };

    let mut page_token: Option<String> = None;
    loop {
        let ls_req = Object::list(
            oid.bucket(),
            Some(ListOptional {
                delimiter,
                page_token: page_token.as_ref().map(|pt| pt.as_ref()),
                prefix: prefix.as_ref().map(|s| s.as_ref()),
                standard_params: StandardQueryParameters {
                    fields: Some(fields),
                    ..Default::default()
                },
                ..Default::default()
            }),
        )?;

        let ls_res: ListResponse = util::execute(ctx, ls_req)?;

        if let Some(ref np) = normal {
            np.print(ls_res.objects, ls_res.prefixes);
        }
        // } else if let Some(ref mut rec) = recurse {
        //     rec.print(ls_res.objects);
        // }

        // If we have a page token it means there may be more items
        // that fulfill the parameters
        page_token = ls_res.page_token;
        if page_token.is_none() {
            break;
        }
    }

    Ok(())
}

#[derive(Copy, Clone)]
enum Display {
    Normal,
    Long,
}

struct NormalPrinter {
    display: Display,
    prefix_len: usize,
}

fn print_dir(display: Display, dir: &str) {
    match display {
        Display::Normal => println!("{}", Color::Blue.bold().paint(dir)),
        Display::Long => println!(
            "    {} {} {} {}",
            Color::White.dimmed().paint("-"),
            Color::White.dimmed().paint("  -"),
            Color::White.dimmed().paint("-- --- --:--:--"),
            Color::Blue.bold().paint(dir),
        ),
    }
}

impl NormalPrinter {
    fn print(&self, items: Vec<Metadata>, prefixes: Vec<String>) {
        let indices = {
            // Determine at which indices we should place the "directories"
            let mut indices = Vec::with_capacity(prefixes.len());

            // So yah...just assume these are always in sorted order...
            for prefix in &prefixes {
                if let Err(i) = items.binary_search_by(|om| om.name.as_ref().unwrap().cmp(&prefix))
                {
                    indices.push(i);
                }
            }

            indices
        };

        let mut next_dir_iter = indices.iter().enumerate();
        let mut next_dir = next_dir_iter.next();

        for (i, item) in items.into_iter().enumerate() {
            if let Some(nd) = next_dir {
                if *nd.1 == i {
                    let dir = &(&prefixes[nd.0])[self.prefix_len..];
                    let dir = &dir[..dir.len() - 1]; // Remove trailing delimiter

                    print_dir(self.display, dir);

                    std::mem::replace(&mut next_dir, next_dir_iter.next());
                }
            }

            let filename = &item.name.unwrap()[self.prefix_len..];

            match self.display {
                Display::Normal => println!("{}", Color::White.paint(filename)),
                Display::Long => {
                    use number_prefix::{NumberPrefix, PrefixNames, Prefixed, Standalone};

                    let size_str = match NumberPrefix::decimal(item.size.unwrap_or_default() as f64)
                    {
                        Standalone(b) => b.to_string(),
                        Prefixed(p, n) => {
                            if n < 10f64 {
                                format!("{:.1}{}", n, p.symbol())
                            } else {
                                format!("{:.0}{}", n, p.symbol())
                            }
                        }
                    };

                    let updated = item.updated.unwrap();
                    let updated_str = updated.format("%d %b %T").to_string();

                    println!(
                        " {}{} {} {} {}",
                        if size_str.len() < 4 { " " } else { "" },
                        Color::Green.paint(size_str),
                        Color::Yellow.paint("gcs"),
                        Color::Blue.paint(updated_str),
                        Color::White.paint(filename),
                    );
                }
            }
        }

        while let Some(nd) = next_dir {
            let dir = &(&prefixes[nd.0])[self.prefix_len..];
            let dir = &dir[..dir.len() - 1]; // Remove trailing delimiter

            print_dir(self.display, dir);

            std::mem::replace(&mut next_dir, next_dir_iter.next());
        }
    }
}

// struct RecursePrinter {
//     display: Display,
//     prefix_len: usize,
//     dir_stack: Vec<String>,
// }

// impl RecursePrinter {
//     fn print(&mut self, items: Vec<Metadata>) {
//         for item in items {
//             let filename = &item.name.unwrap()[self.prefix_len..];

//             self.write_dirs(filename);
//         }
//     }

//     fn write_dirs(&mut self, filename: &str) {
//         if let Some(sep) = filename.rfind('/') {
//             let filename = &filename[..sep];

//             let mut dir_name = String::new();
//             for part in filename.split('/') {
//                 dir_name.push_str(part);

//                 if !self.dirs.contains(&dir_name) {
//                 }
//             }
//         }
//     }
// }
