import '@/Styles/Header.css';

interface HeaderProps {
    sections: string[];
}

export default function Header({ sections }: HeaderProps){
    return (
        <header className="fixed flex justify-between w-full h-14 px-10 bg-darkboard items-center text-white drop-shadow-2xl z-20">
            <div className="w-10 h-10 rounded-full bg-yellow-chalk"/>
            <div className="h-full">
                <ul className="flex h-full">
                    {sections.map((section, index) => <li
                        className="relative section h-full w-20 mx-1 p-1 flex items-center justify-center"
                        key={index} 
                    >
                        <a href={`#${section}`}>
                            {section}
                        </a>
                    </li>)}
                </ul>
            </div>
        </header>
    );
}