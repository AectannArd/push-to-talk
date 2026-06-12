interface Props {
  code: 'en-US' | 'ru-RU';
}

/** Renders a small rounded flag image. */
export default function FlagIcon({ code }: Props) {
  const src =
    code === 'en-US'
      ? 'data:image/svg+xml,' + encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="24" height="18" fill="#B22234"/><g stroke="#fff" stroke-width="1.6"><line x1="0" y1="2.25" x2="24" y2="2.25"/><line x1="0" y1="6.75" x2="24" y2="6.75"/><line x1="0" y1="11.25" x2="24" y2="11.25"/><line x1="0" y1="15.75" x2="24" y2="15.75"/></g><rect width="10" height="9.75" fill="#3C3B6E"/></svg>')
      : 'data:image/svg+xml,' + encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="24" height="6" fill="#fff"/><rect y="6" width="24" height="6" fill="#0039A6"/><rect y="12" width="24" height="6" fill="#D52B1E"/></svg>');

  return (
    <img
      src={src}
      alt={code}
      style={{
        width: 18,
        height: 13,
        borderRadius: 2,
        verticalAlign: 'middle',
        marginRight: 5,
        flexShrink: 0,
      }}
    />
  );
}
