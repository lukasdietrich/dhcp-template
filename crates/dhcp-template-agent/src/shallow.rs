use dhcp_template_api::{Shallow, Update, update::Data};

pub trait ShallowClone {
    fn shallow_clone(&self) -> Self;
}

impl ShallowClone for Update {
    fn shallow_clone(&self) -> Self {
        Update {
            token: self.token,
            data: match &self.data {
                Some(Data::Full(node)) => Some(Data::Shallow(Shallow {
                    name: node.name.clone(),
                })),
                shallow => shallow.clone(),
            },
        }
    }
}
