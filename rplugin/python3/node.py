class Node:
    def __init__(self, node_id, directory, title, parent) -> None:
        self.id = node_id
        self.directory = directory
        self.title = title
        self.parent = parent
        self.children = []
        self.links = []
        self.backlinks = []
        self.tags = []
