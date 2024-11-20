// @ts-nocheck
/* eslint-disable */
import { process_exports, } from './chunk-RUAN5HWR.js';

// https :https://framerusercontent.com/modules/wFkXxZqfKOPeEQtsYrsa/Xcw7qvncyogRPQtf9aNn/codemirror_state.js
var Text = class {
  /**
  Get the line description around the given position.
  */
  lineAt(pos,) {
    if (pos < 0 || pos > this.length) throw new RangeError(`Invalid position ${pos} in document of length ${this.length}`,);
    return this.lineInner(pos, false, 1, 0,);
  }
  /**
  Get the description for the given (1-based) line number.
  */
  line(n,) {
    if (n < 1 || n > this.lines) throw new RangeError(`Invalid line number ${n} in ${this.lines}-line document`,);
    return this.lineInner(n, true, 1, 0,);
  }
  /**
  Replace a range of the text with the given content.
  */
  replace(from, to, text,) {
    let parts = [];
    this.decompose(0, from, parts, 2,);
    if (text.length) text.decompose(0, text.length, parts, 1 | 2,);
    this.decompose(to, this.length, parts, 1,);
    return TextNode.from(parts, this.length - (to - from) + text.length,);
  }
  /**
  Append another document to this one.
  */
  append(other,) {
    return this.replace(this.length, this.length, other,);
  }
  /**
  Retrieve the text between the given points.
  */
  slice(from, to = this.length,) {
    let parts = [];
    this.decompose(from, to, parts, 0,);
    return TextNode.from(parts, to - from,);
  }
  /**
  Test whether this text is equal to another instance.
  */
  eq(other,) {
    if (other == this) return true;
    if (other.length != this.length || other.lines != this.lines) return false;
    let start = this.scanIdentical(other, 1,), end = this.length - this.scanIdentical(other, -1,);
    let a = new RawTextCursor(this,), b = new RawTextCursor(other,);
    for (let skip = start, pos = start;;) {
      a.next(skip,);
      b.next(skip,);
      skip = 0;
      if (a.lineBreak != b.lineBreak || a.done != b.done || a.value != b.value) return false;
      pos += a.value.length;
      if (a.done || pos >= end) return true;
    }
  }
  /**
  Iterate over the text. When `dir` is `-1`, iteration happens
  from end to start. This will return lines and the breaks between
  them as separate strings.
  */
  iter(dir = 1,) {
    return new RawTextCursor(this, dir,);
  }
  /**
  Iterate over a range of the text. When `from` > `to`, the
  iterator will run in reverse.
  */
  iterRange(from, to = this.length,) {
    return new PartialTextCursor(this, from, to,);
  }
  /**
  Return a cursor that iterates over the given range of lines,
  _without_ returning the line breaks between, and yielding empty
  strings for empty lines.

  When `from` and `to` are given, they should be 1-based line numbers.
  */
  iterLines(from, to,) {
    let inner;
    if (from == null) {
      inner = this.iter();
    } else {
      if (to == null) to = this.lines + 1;
      let start = this.line(from,).from;
      inner = this.iterRange(start, Math.max(start, to == this.lines + 1 ? this.length : to <= 1 ? 0 : this.line(to - 1,).to,),);
    }
    return new LineCursor(inner,);
  }
  /**
  Return the document as a string, using newline characters to
  separate lines.
  */
  toString() {
    return this.sliceString(0,);
  }
  /**
  Convert the document to an array of lines (which can be
  deserialized again via [`Text.of`](https://codemirror.net/6/docs/ref/#state.Text^of)).
  */
  toJSON() {
    let lines = [];
    this.flatten(lines,);
    return lines;
  }
  /**
  Create a `Text` instance for the given array of lines.
  */
  static of(text,) {
    if (text.length == 0) throw new RangeError('A document must have at least one line',);
    if (text.length == 1 && !text[0]) return Text.empty;
    return text.length <= 32 ? new TextLeaf(text,) : TextNode.from(TextLeaf.split(text, [],),);
  }
  /**
  @internal
  */
  constructor() {
  }
};
var TextLeaf = class extends Text {
  get lines() {
    return this.text.length;
  }
  get children() {
    return null;
  }
  lineInner(target, isLine, line, offset,) {
    for (let i2 = 0;; i2++) {
      let string2 = this.text[i2], end = offset + string2.length;
      if ((isLine ? line : end) >= target) return new Line(offset, end, line, string2,);
      offset = end + 1;
      line++;
    }
  }
  decompose(from, to, target, open,) {
    let text = from <= 0 && to >= this.length
      ? this
      : new TextLeaf(sliceText(this.text, from, to,), Math.min(to, this.length,) - Math.max(0, from,),);
    if (open & 1) {
      let prev = target.pop();
      let joined = appendText(text.text, prev.text.slice(), 0, text.length,);
      if (joined.length <= 32) {
        target.push(new TextLeaf(joined, prev.length + text.length,),);
      } else {
        let mid = joined.length >> 1;
        target.push(new TextLeaf(joined.slice(0, mid,),), new TextLeaf(joined.slice(mid,),),);
      }
    } else {
      target.push(text,);
    }
  }
  replace(from, to, text,) {
    if (!(text instanceof TextLeaf)) return super.replace(from, to, text,);
    let lines = appendText(this.text, appendText(text.text, sliceText(this.text, 0, from,),), to,);
    let newLen = this.length + text.length - (to - from);
    if (lines.length <= 32) return new TextLeaf(lines, newLen,);
    return TextNode.from(TextLeaf.split(lines, [],), newLen,);
  }
  sliceString(from, to = this.length, lineSep = '\n',) {
    let result = '';
    for (let pos = 0, i2 = 0; pos <= to && i2 < this.text.length; i2++) {
      let line = this.text[i2], end = pos + line.length;
      if (pos > from && i2) result += lineSep;
      if (from < end && to > pos) result += line.slice(Math.max(0, from - pos,), to - pos,);
      pos = end + 1;
    }
    return result;
  }
  flatten(target,) {
    for (let line of this.text) target.push(line,);
  }
  scanIdentical() {
    return 0;
  }
  static split(text, target,) {
    let part = [], len = -1;
    for (let line of text) {
      part.push(line,);
      len += line.length + 1;
      if (part.length == 32) {
        target.push(new TextLeaf(part, len,),);
        part = [];
        len = -1;
      }
    }
    if (len > -1) target.push(new TextLeaf(part, len,),);
    return target;
  }
  constructor(text, length = textLength(text,),) {
    super();
    this.text = text;
    this.length = length;
  }
};
var TextNode = class extends Text {
  lineInner(target, isLine, line, offset,) {
    for (let i2 = 0;; i2++) {
      let child = this.children[i2], end = offset + child.length, endLine = line + child.lines - 1;
      if ((isLine ? endLine : end) >= target) return child.lineInner(target, isLine, line, offset,);
      offset = end + 1;
      line = endLine + 1;
    }
  }
  decompose(from, to, target, open,) {
    for (let i2 = 0, pos = 0; pos <= to && i2 < this.children.length; i2++) {
      let child = this.children[i2], end = pos + child.length;
      if (from <= end && to >= pos) {
        let childOpen = open & ((pos <= from ? 1 : 0) | (end >= to ? 2 : 0));
        if (pos >= from && end <= to && !childOpen) target.push(child,);
        else child.decompose(from - pos, to - pos, target, childOpen,);
      }
      pos = end + 1;
    }
  }
  replace(from, to, text,) {
    if (text.lines < this.lines) {
      for (let i2 = 0, pos = 0; i2 < this.children.length; i2++) {
        let child = this.children[i2], end = pos + child.length;
        if (from >= pos && to <= end) {
          let updated = child.replace(from - pos, to - pos, text,);
          let totalLines = this.lines - child.lines + updated.lines;
          if (updated.lines < totalLines >> 5 - 1 && updated.lines > totalLines >> 5 + 1) {
            let copy = this.children.slice();
            copy[i2] = updated;
            return new TextNode(copy, this.length - (to - from) + text.length,);
          }
          return super.replace(pos, end, updated,);
        }
        pos = end + 1;
      }
    }
    return super.replace(from, to, text,);
  }
  sliceString(from, to = this.length, lineSep = '\n',) {
    let result = '';
    for (let i2 = 0, pos = 0; i2 < this.children.length && pos <= to; i2++) {
      let child = this.children[i2], end = pos + child.length;
      if (pos > from && i2) result += lineSep;
      if (from < end && to > pos) result += child.sliceString(from - pos, to - pos, lineSep,);
      pos = end + 1;
    }
    return result;
  }
  flatten(target,) {
    for (let child of this.children) child.flatten(target,);
  }
  scanIdentical(other, dir,) {
    if (!(other instanceof TextNode)) return 0;
    let length = 0;
    let [iA, iB, eA, eB,] = dir > 0
      ? [0, 0, this.children.length, other.children.length,]
      : [this.children.length - 1, other.children.length - 1, -1, -1,];
    for (;; iA += dir, iB += dir) {
      if (iA == eA || iB == eB) return length;
      let chA = this.children[iA], chB = other.children[iB];
      if (chA != chB) return length + chA.scanIdentical(chB, dir,);
      length += chA.length + 1;
    }
  }
  static from(children, length = children.reduce((l, ch,) => l + ch.length + 1, -1,),) {
    let lines = 0;
    for (let ch of children) lines += ch.lines;
    if (lines < 32) {
      let flat = [];
      for (let ch1 of children) ch1.flatten(flat,);
      return new TextLeaf(flat, length,);
    }
    let chunk = Math.max(32, lines >> 5,), maxChunk = chunk << 1, minChunk = chunk >> 1;
    let chunked = [], currentLines = 0, currentLen = -1, currentChunk = [];
    function add(child,) {
      let last;
      if (child.lines > maxChunk && child instanceof TextNode) {
        for (let node of child.children) add(node,);
      } else if (child.lines > minChunk && (currentLines > minChunk || !currentLines)) {
        flush();
        chunked.push(child,);
      } else if (
        child instanceof TextLeaf && currentLines && (last = currentChunk[currentChunk.length - 1]) instanceof TextLeaf &&
        child.lines + last.lines <= 32
      ) {
        currentLines += child.lines;
        currentLen += child.length + 1;
        currentChunk[currentChunk.length - 1] = new TextLeaf(last.text.concat(child.text,), last.length + 1 + child.length,);
      } else {
        if (currentLines + child.lines > chunk) flush();
        currentLines += child.lines;
        currentLen += child.length + 1;
        currentChunk.push(child,);
      }
    }
    function flush() {
      if (currentLines == 0) return;
      chunked.push(currentChunk.length == 1 ? currentChunk[0] : TextNode.from(currentChunk, currentLen,),);
      currentLen = -1;
      currentLines = currentChunk.length = 0;
    }
    for (let child of children) add(child,);
    flush();
    return chunked.length == 1 ? chunked[0] : new TextNode(chunked, length,);
  }
  constructor(children, length,) {
    super();
    this.children = children;
    this.length = length;
    this.lines = 0;
    for (let child of children) this.lines += child.lines;
  }
};
Text.empty = /* @__PURE__ */ new TextLeaf(['',], 0,);
function textLength(text,) {
  let length = -1;
  for (let line of text) length += line.length + 1;
  return length;
}
function appendText(text, target, from = 0, to = 1e9,) {
  for (let pos = 0, i2 = 0, first = true; i2 < text.length && pos <= to; i2++) {
    let line = text[i2], end = pos + line.length;
    if (end >= from) {
      if (end > to) line = line.slice(0, to - pos,);
      if (pos < from) line = line.slice(from - pos,);
      if (first) {
        target[target.length - 1] += line;
        first = false;
      } else target.push(line,);
    }
    pos = end + 1;
  }
  return target;
}
function sliceText(text, from, to,) {
  return appendText(text, ['',], from, to,);
}
var RawTextCursor = class {
  nextInner(skip, dir,) {
    this.done = this.lineBreak = false;
    for (;;) {
      let last = this.nodes.length - 1;
      let top3 = this.nodes[last], offsetValue = this.offsets[last], offset = offsetValue >> 1;
      let size = top3 instanceof TextLeaf ? top3.text.length : top3.children.length;
      if (offset == (dir > 0 ? size : 0)) {
        if (last == 0) {
          this.done = true;
          this.value = '';
          return this;
        }
        if (dir > 0) this.offsets[last - 1]++;
        this.nodes.pop();
        this.offsets.pop();
      } else if ((offsetValue & 1) == (dir > 0 ? 0 : 1)) {
        this.offsets[last] += dir;
        if (skip == 0) {
          this.lineBreak = true;
          this.value = '\n';
          return this;
        }
        skip--;
      } else if (top3 instanceof TextLeaf) {
        let next = top3.text[offset + (dir < 0 ? -1 : 0)];
        this.offsets[last] += dir;
        if (next.length > Math.max(0, skip,)) {
          this.value = skip == 0 ? next : dir > 0 ? next.slice(skip,) : next.slice(0, next.length - skip,);
          return this;
        }
        skip -= next.length;
      } else {
        let next1 = top3.children[offset + (dir < 0 ? -1 : 0)];
        if (skip > next1.length) {
          skip -= next1.length;
          this.offsets[last] += dir;
        } else {
          if (dir < 0) this.offsets[last]--;
          this.nodes.push(next1,);
          this.offsets.push(dir > 0 ? 1 : (next1 instanceof TextLeaf ? next1.text.length : next1.children.length) << 1,);
        }
      }
    }
  }
  next(skip = 0,) {
    if (skip < 0) {
      this.nextInner(-skip, -this.dir,);
      skip = this.value.length;
    }
    return this.nextInner(skip, this.dir,);
  }
  constructor(text, dir = 1,) {
    this.dir = dir;
    this.done = false;
    this.lineBreak = false;
    this.value = '';
    this.nodes = [text,];
    this.offsets = [dir > 0 ? 1 : (text instanceof TextLeaf ? text.text.length : text.children.length) << 1,];
  }
};
var PartialTextCursor = class {
  nextInner(skip, dir,) {
    if (dir < 0 ? this.pos <= this.from : this.pos >= this.to) {
      this.value = '';
      this.done = true;
      return this;
    }
    skip += Math.max(0, dir < 0 ? this.pos - this.to : this.from - this.pos,);
    let limit = dir < 0 ? this.pos - this.from : this.to - this.pos;
    if (skip > limit) skip = limit;
    limit -= skip;
    let { value, } = this.cursor.next(skip,);
    this.pos += (value.length + skip) * dir;
    this.value = value.length <= limit ? value : dir < 0 ? value.slice(value.length - limit,) : value.slice(0, limit,);
    this.done = !this.value;
    return this;
  }
  next(skip = 0,) {
    if (skip < 0) skip = Math.max(skip, this.from - this.pos,);
    else if (skip > 0) skip = Math.min(skip, this.to - this.pos,);
    return this.nextInner(skip, this.cursor.dir,);
  }
  get lineBreak() {
    return this.cursor.lineBreak && this.value != '';
  }
  constructor(text, start, end,) {
    this.value = '';
    this.done = false;
    this.cursor = new RawTextCursor(text, start > end ? -1 : 1,);
    this.pos = start > end ? text.length : 0;
    this.from = Math.min(start, end,);
    this.to = Math.max(start, end,);
  }
};
var LineCursor = class {
  next(skip = 0,) {
    let { done, lineBreak, value, } = this.inner.next(skip,);
    if (done) {
      this.done = true;
      this.value = '';
    } else if (lineBreak) {
      if (this.afterBreak) {
        this.value = '';
      } else {
        this.afterBreak = true;
        this.next();
      }
    } else {
      this.value = value;
      this.afterBreak = false;
    }
    return this;
  }
  get lineBreak() {
    return false;
  }
  constructor(inner,) {
    this.inner = inner;
    this.afterBreak = true;
    this.value = '';
    this.done = false;
  }
};
if (typeof Symbol != 'undefined') {
  Text.prototype[Symbol.iterator] = function () {
    return this.iter();
  };
  RawTextCursor.prototype[Symbol.iterator] =
    PartialTextCursor.prototype[Symbol.iterator] =
    LineCursor.prototype[Symbol.iterator] =
      function () {
        return this;
      };
}
var Line = class {
  /**
  The length of the line (not including any line break after it).
  */
  get length() {
    return this.to - this.from;
  }
  /**
  @internal
  */
  constructor(from, to, number2, text,) {
    this.from = from;
    this.to = to;
    this.number = number2;
    this.text = text;
  }
};
var extend =
  /* @__PURE__ */ 'lc,34,7n,7,7b,19,,,,2,,2,,,20,b,1c,l,g,,2t,7,2,6,2,2,,4,z,,u,r,2j,b,1m,9,9,,o,4,,9,,3,,5,17,3,3b,f,,w,1j,,,,4,8,4,,3,7,a,2,t,,1m,,,,2,4,8,,9,,a,2,q,,2,2,1l,,4,2,4,2,2,3,3,,u,2,3,,b,2,1l,,4,5,,2,4,,k,2,m,6,,,1m,,,2,,4,8,,7,3,a,2,u,,1n,,,,c,,9,,14,,3,,1l,3,5,3,,4,7,2,b,2,t,,1m,,2,,2,,3,,5,2,7,2,b,2,s,2,1l,2,,,2,4,8,,9,,a,2,t,,20,,4,,2,3,,,8,,29,,2,7,c,8,2q,,2,9,b,6,22,2,r,,,,,,1j,e,,5,,2,5,b,,10,9,,2u,4,,6,,2,2,2,p,2,4,3,g,4,d,,2,2,6,,f,,jj,3,qa,3,t,3,t,2,u,2,1s,2,,7,8,,2,b,9,,19,3,3b,2,y,,3a,3,4,2,9,,6,3,63,2,2,,1m,,,7,,,,,2,8,6,a,2,,1c,h,1r,4,1c,7,,,5,,14,9,c,2,w,4,2,2,,3,1k,,,2,3,,,3,1m,8,2,2,48,3,,d,,7,4,,6,,3,2,5i,1m,,5,ek,,5f,x,2da,3,3x,,2o,w,fe,6,2x,2,n9w,4,,a,w,2,28,2,7k,,3,,4,,p,2,5,,47,2,q,i,d,,12,8,p,b,1a,3,1c,,2,4,2,2,13,,1v,6,2,2,2,2,c,,8,,1b,,1f,,,3,2,2,5,2,,,16,2,8,,6m,,2,,4,,fn4,,kh,g,g,g,a6,2,gt,,6a,,45,5,1ae,3,,2,5,4,14,3,4,,4l,2,fx,4,ar,2,49,b,4w,,1i,f,1k,3,1d,4,2,2,1x,3,10,5,,8,1q,,c,2,1g,9,a,4,2,,2n,3,2,,,2,6,,4g,,3,8,l,2,1l,2,,,,,m,,e,7,3,5,5f,8,2,3,,,n,,29,,2,6,,,2,,,2,,2,6j,,2,4,6,2,,2,r,2,2d,8,2,,,2,2y,,,,2,6,,,2t,3,2,4,,5,77,9,,2,6t,,a,2,,,4,,40,4,2,2,4,,w,a,14,6,2,4,8,,9,6,2,3,1a,d,,2,ba,7,,6,,,2a,m,2,7,,2,,2,3e,6,3,,,2,,7,,,20,2,3,,,,9n,2,f0b,5,1n,7,t4,,1r,4,29,,f5k,2,43q,,,3,4,5,8,8,2,7,u,4,44,3,1iz,1j,4,1e,8,,e,,m,5,,f,11s,7,,h,2,7,,2,,5,79,7,c5,4,15s,7,31,7,240,5,gx7k,2o,3k,6o'
    .split(',',).map((s,) => s ? parseInt(s, 36,) : 1);
for (let i2 = 1; i2 < extend.length; i2++) extend[i2] += extend[i2 - 1];
function isExtendingChar(code2,) {
  for (let i2 = 1; i2 < extend.length; i2 += 2) if (extend[i2] > code2) return extend[i2 - 1] <= code2;
  return false;
}
function isRegionalIndicator(code2,) {
  return code2 >= 127462 && code2 <= 127487;
}
var ZWJ = 8205;
function findClusterBreak(str, pos, forward = true, includeExtending = true,) {
  return (forward ? nextClusterBreak : prevClusterBreak)(str, pos, includeExtending,);
}
function nextClusterBreak(str, pos, includeExtending,) {
  if (pos == str.length) return pos;
  if (pos && surrogateLow(str.charCodeAt(pos,),) && surrogateHigh(str.charCodeAt(pos - 1,),)) pos--;
  let prev = codePointAt(str, pos,);
  pos += codePointSize(prev,);
  while (pos < str.length) {
    let next = codePointAt(str, pos,);
    if (prev == ZWJ || next == ZWJ || includeExtending && isExtendingChar(next,)) {
      pos += codePointSize(next,);
      prev = next;
    } else if (isRegionalIndicator(next,)) {
      let countBefore = 0, i2 = pos - 2;
      while (i2 >= 0 && isRegionalIndicator(codePointAt(str, i2,),)) {
        countBefore++;
        i2 -= 2;
      }
      if (countBefore % 2 == 0) break;
      else pos += 2;
    } else {
      break;
    }
  }
  return pos;
}
function prevClusterBreak(str, pos, includeExtending,) {
  while (pos > 0) {
    let found = nextClusterBreak(str, pos - 2, includeExtending,);
    if (found < pos) return found;
    pos--;
  }
  return 0;
}
function surrogateLow(ch,) {
  return ch >= 56320 && ch < 57344;
}
function surrogateHigh(ch,) {
  return ch >= 55296 && ch < 56320;
}
function codePointAt(str, pos,) {
  let code0 = str.charCodeAt(pos,);
  if (!surrogateHigh(code0,) || pos + 1 == str.length) return code0;
  let code1 = str.charCodeAt(pos + 1,);
  if (!surrogateLow(code1,)) return code0;
  return (code0 - 55296 << 10) + (code1 - 56320) + 65536;
}
function fromCodePoint(code2,) {
  if (code2 <= 65535) return String.fromCharCode(code2,);
  code2 -= 65536;
  return String.fromCharCode((code2 >> 10) + 55296, (code2 & 1023) + 56320,);
}
function codePointSize(code2,) {
  return code2 < 65536 ? 1 : 2;
}
var DefaultSplit = /\r\n?|\n/;
var MapMode = /* @__PURE__ */ function (MapMode2,) {
  MapMode2[MapMode2['Simple'] = 0] = 'Simple';
  MapMode2[MapMode2['TrackDel'] = 1] = 'TrackDel';
  MapMode2[MapMode2['TrackBefore'] = 2] = 'TrackBefore';
  MapMode2[MapMode2['TrackAfter'] = 3] = 'TrackAfter';
  return MapMode2;
}(MapMode || (MapMode = {}),);
var ChangeDesc = class {
  /**
  The length of the document before the change.
  */
  get length() {
    let result = 0;
    for (let i2 = 0; i2 < this.sections.length; i2 += 2) result += this.sections[i2];
    return result;
  }
  /**
  The length of the document after the change.
  */
  get newLength() {
    let result = 0;
    for (let i2 = 0; i2 < this.sections.length; i2 += 2) {
      let ins = this.sections[i2 + 1];
      result += ins < 0 ? this.sections[i2] : ins;
    }
    return result;
  }
  /**
  False when there are actual changes in this set.
  */
  get empty() {
    return this.sections.length == 0 || this.sections.length == 2 && this.sections[1] < 0;
  }
  /**
  Iterate over the unchanged parts left by these changes. `posA`
  provides the position of the range in the old document, `posB`
  the new position in the changed document.
  */
  iterGaps(f,) {
    for (let i2 = 0, posA = 0, posB = 0; i2 < this.sections.length;) {
      let len = this.sections[i2++], ins = this.sections[i2++];
      if (ins < 0) {
        f(posA, posB, len,);
        posB += len;
      } else {
        posB += ins;
      }
      posA += len;
    }
  }
  /**
  Iterate over the ranges changed by these changes. (See
  [`ChangeSet.iterChanges`](https://codemirror.net/6/docs/ref/#state.ChangeSet.iterChanges) for a
  variant that also provides you with the inserted text.)
  `fromA`/`toA` provides the extent of the change in the starting
  document, `fromB`/`toB` the extent of the replacement in the
  changed document.

  When `individual` is true, adjacent changes (which are kept
  separate for [position mapping](https://codemirror.net/6/docs/ref/#state.ChangeDesc.mapPos)) are
  reported separately.
  */
  iterChangedRanges(f, individual = false,) {
    iterChanges(this, f, individual,);
  }
  /**
  Get a description of the inverted form of these changes.
  */
  get invertedDesc() {
    let sections = [];
    for (let i2 = 0; i2 < this.sections.length;) {
      let len = this.sections[i2++], ins = this.sections[i2++];
      if (ins < 0) sections.push(len, ins,);
      else sections.push(ins, len,);
    }
    return new ChangeDesc(sections,);
  }
  /**
  Compute the combined effect of applying another set of changes
  after this one. The length of the document after this set should
  match the length before `other`.
  */
  composeDesc(other,) {
    return this.empty ? other : other.empty ? this : composeSets(this, other,);
  }
  /**
  Map this description, which should start with the same document
  as `other`, over another set of changes, so that it can be
  applied after it. When `before` is true, map as if the changes
  in `other` happened before the ones in `this`.
  */
  mapDesc(other, before = false,) {
    return other.empty ? this : mapSet(this, other, before,);
  }
  mapPos(pos, assoc = -1, mode = MapMode.Simple,) {
    let posA = 0, posB = 0;
    for (let i2 = 0; i2 < this.sections.length;) {
      let len = this.sections[i2++], ins = this.sections[i2++], endA = posA + len;
      if (ins < 0) {
        if (endA > pos) return posB + (pos - posA);
        posB += len;
      } else {
        if (
          mode != MapMode.Simple && endA >= pos &&
          (mode == MapMode.TrackDel && posA < pos && endA > pos || mode == MapMode.TrackBefore && posA < pos ||
            mode == MapMode.TrackAfter && endA > pos)
        ) return null;
        if (endA > pos || endA == pos && assoc < 0 && !len) return pos == posA || assoc < 0 ? posB : posB + ins;
        posB += ins;
      }
      posA = endA;
    }
    if (pos > posA) throw new RangeError(`Position ${pos} is out of range for changeset of length ${posA}`,);
    return posB;
  }
  /**
  Check whether these changes touch a given range. When one of the
  changes entirely covers the range, the string `"cover"` is
  returned.
  */
  touchesRange(from, to = from,) {
    for (let i2 = 0, pos = 0; i2 < this.sections.length && pos <= to;) {
      let len = this.sections[i2++], ins = this.sections[i2++], end = pos + len;
      if (ins >= 0 && pos <= to && end >= from) return pos < from && end > to ? 'cover' : true;
      pos = end;
    }
    return false;
  }
  /**
  @internal
  */
  toString() {
    let result = '';
    for (let i2 = 0; i2 < this.sections.length;) {
      let len = this.sections[i2++], ins = this.sections[i2++];
      result += (result ? ' ' : '') + len + (ins >= 0 ? ':' + ins : '');
    }
    return result;
  }
  /**
  Serialize this change desc to a JSON-representable value.
  */
  toJSON() {
    return this.sections;
  }
  /**
  Create a change desc from its JSON representation (as produced
  by [`toJSON`](https://codemirror.net/6/docs/ref/#state.ChangeDesc.toJSON).
  */
  static fromJSON(json,) {
    if (!Array.isArray(json,) || json.length % 2 || json.some((a,) => typeof a != 'number')) {
      throw new RangeError('Invalid JSON representation of ChangeDesc',);
    }
    return new ChangeDesc(json,);
  }
  /**
  @internal
  */
  static create(sections,) {
    return new ChangeDesc(sections,);
  }
  // Sections are encoded as pairs of integers. The first is the
  // length in the current document, and the second is -1 for
  // unaffected sections, and the length of the replacement content
  // otherwise. So an insertion would be (0, n>0), a deletion (n>0,
  // 0), and a replacement two positive numbers.
  /**
    @internal
    */
  constructor(sections,) {
    this.sections = sections;
  }
};
var ChangeSet = class extends ChangeDesc {
  /**
  Apply the changes to a document, returning the modified
  document.
  */
  apply(doc2,) {
    if (this.length != doc2.length) throw new RangeError('Applying change set to a document with the wrong length',);
    iterChanges(this, (fromA, toA, fromB, _toB, text,) => doc2 = doc2.replace(fromB, fromB + (toA - fromA), text,), false,);
    return doc2;
  }
  mapDesc(other, before = false,) {
    return mapSet(this, other, before, true,);
  }
  /**
  Given the document as it existed _before_ the changes, return a
  change set that represents the inverse of this set, which could
  be used to go from the document created by the changes back to
  the document as it existed before the changes.
  */
  invert(doc2,) {
    let sections = this.sections.slice(), inserted = [];
    for (let i2 = 0, pos = 0; i2 < sections.length; i2 += 2) {
      let len = sections[i2], ins = sections[i2 + 1];
      if (ins >= 0) {
        sections[i2] = ins;
        sections[i2 + 1] = len;
        let index = i2 >> 1;
        while (inserted.length < index) inserted.push(Text.empty,);
        inserted.push(len ? doc2.slice(pos, pos + len,) : Text.empty,);
      }
      pos += len;
    }
    return new ChangeSet(sections, inserted,);
  }
  /**
  Combine two subsequent change sets into a single set. `other`
  must start in the document produced by `this`. If `this` goes
  `docA` → `docB` and `other` represents `docB` → `docC`, the
  returned value will represent the change `docA` → `docC`.
  */
  compose(other,) {
    return this.empty ? other : other.empty ? this : composeSets(this, other, true,);
  }
  /**
  Given another change set starting in the same document, maps this
  change set over the other, producing a new change set that can be
  applied to the document produced by applying `other`. When
  `before` is `true`, order changes as if `this` comes before
  `other`, otherwise (the default) treat `other` as coming first.

  Given two changes `A` and `B`, `A.compose(B.map(A))` and
  `B.compose(A.map(B, true))` will produce the same document. This
  provides a basic form of [operational
  transformation](https://en.wikipedia.org/wiki/Operational_transformation),
  and can be used for collaborative editing.
  */
  map(other, before = false,) {
    return other.empty ? this : mapSet(this, other, before, true,);
  }
  /**
  Iterate over the changed ranges in the document, calling `f` for
  each, with the range in the original document (`fromA`-`toA`)
  and the range that replaces it in the new document
  (`fromB`-`toB`).

  When `individual` is true, adjacent changes are reported
  separately.
  */
  iterChanges(f, individual = false,) {
    iterChanges(this, f, individual,);
  }
  /**
  Get a [change description](https://codemirror.net/6/docs/ref/#state.ChangeDesc) for this change
  set.
  */
  get desc() {
    return ChangeDesc.create(this.sections,);
  }
  /**
  @internal
  */
  filter(ranges,) {
    let resultSections = [], resultInserted = [], filteredSections = [];
    let iter = new SectionIter(this,);
    done: for (let i2 = 0, pos = 0;;) {
      let next = i2 == ranges.length ? 1e9 : ranges[i2++];
      while (pos < next || pos == next && iter.len == 0) {
        if (iter.done) break done;
        let len = Math.min(iter.len, next - pos,);
        addSection(filteredSections, len, -1,);
        let ins = iter.ins == -1 ? -1 : iter.off == 0 ? iter.ins : 0;
        addSection(resultSections, len, ins,);
        if (ins > 0) addInsert(resultInserted, resultSections, iter.text,);
        iter.forward(len,);
        pos += len;
      }
      let end = ranges[i2++];
      while (pos < end) {
        if (iter.done) break done;
        let len1 = Math.min(iter.len, end - pos,);
        addSection(resultSections, len1, -1,);
        addSection(filteredSections, len1, iter.ins == -1 ? -1 : iter.off == 0 ? iter.ins : 0,);
        iter.forward(len1,);
        pos += len1;
      }
    }
    return { changes: new ChangeSet(resultSections, resultInserted,), filtered: ChangeDesc.create(filteredSections,), };
  }
  /**
  Serialize this change set to a JSON-representable value.
  */
  toJSON() {
    let parts = [];
    for (let i2 = 0; i2 < this.sections.length; i2 += 2) {
      let len = this.sections[i2], ins = this.sections[i2 + 1];
      if (ins < 0) parts.push(len,);
      else if (ins == 0) parts.push([len,],);
      else parts.push([len,].concat(this.inserted[i2 >> 1].toJSON(),),);
    }
    return parts;
  }
  /**
  Create a change set for the given changes, for a document of the
  given length, using `lineSep` as line separator.
  */
  static of(changes, length, lineSep,) {
    let sections = [], inserted = [], pos = 0;
    let total = null;
    function flush(force = false,) {
      if (!force && !sections.length) return;
      if (pos < length) addSection(sections, length - pos, -1,);
      let set = new ChangeSet(sections, inserted,);
      total = total ? total.compose(set.map(total,),) : set;
      sections = [];
      inserted = [];
      pos = 0;
    }
    function process(spec,) {
      if (Array.isArray(spec,)) {
        for (let sub of spec) process(sub,);
      } else if (spec instanceof ChangeSet) {
        if (spec.length != length) throw new RangeError(`Mismatched change set length (got ${spec.length}, expected ${length})`,);
        flush();
        total = total ? total.compose(spec.map(total,),) : spec;
      } else {
        let { from, to = from, insert: insert2, } = spec;
        if (from > to || from < 0 || to > length) {
          throw new RangeError(`Invalid change range ${from} to ${to} (in doc of length ${length})`,);
        }
        let insText = !insert2 ? Text.empty : typeof insert2 == 'string' ? Text.of(insert2.split(lineSep || DefaultSplit,),) : insert2;
        let insLen = insText.length;
        if (from == to && insLen == 0) return;
        if (from < pos) flush();
        if (from > pos) addSection(sections, from - pos, -1,);
        addSection(sections, to - from, insLen,);
        addInsert(inserted, sections, insText,);
        pos = to;
      }
    }
    process(changes,);
    flush(!total,);
    return total;
  }
  /**
  Create an empty changeset of the given length.
  */
  static empty(length,) {
    return new ChangeSet(length ? [length, -1,] : [], [],);
  }
  /**
  Create a changeset from its JSON representation (as produced by
  [`toJSON`](https://codemirror.net/6/docs/ref/#state.ChangeSet.toJSON).
  */
  static fromJSON(json,) {
    if (!Array.isArray(json,)) throw new RangeError('Invalid JSON representation of ChangeSet',);
    let sections = [], inserted = [];
    for (let i2 = 0; i2 < json.length; i2++) {
      let part = json[i2];
      if (typeof part == 'number') {
        sections.push(part, -1,);
      } else if (!Array.isArray(part,) || typeof part[0] != 'number' || part.some((e, i22,) => i22 && typeof e != 'string')) {
        throw new RangeError('Invalid JSON representation of ChangeSet',);
      } else if (part.length == 1) {
        sections.push(part[0], 0,);
      } else {
        while (inserted.length < i2) inserted.push(Text.empty,);
        inserted[i2] = Text.of(part.slice(1,),);
        sections.push(part[0], inserted[i2].length,);
      }
    }
    return new ChangeSet(sections, inserted,);
  }
  /**
  @internal
  */
  static createSet(sections, inserted,) {
    return new ChangeSet(sections, inserted,);
  }
  constructor(sections, inserted,) {
    super(sections,);
    this.inserted = inserted;
  }
};
function addSection(sections, len, ins, forceJoin = false,) {
  if (len == 0 && ins <= 0) return;
  let last = sections.length - 2;
  if (last >= 0 && ins <= 0 && ins == sections[last + 1]) sections[last] += len;
  else if (len == 0 && sections[last] == 0) sections[last + 1] += ins;
  else if (forceJoin) {
    sections[last] += len;
    sections[last + 1] += ins;
  } else sections.push(len, ins,);
}
function addInsert(values, sections, value,) {
  if (value.length == 0) return;
  let index = sections.length - 2 >> 1;
  if (index < values.length) {
    values[values.length - 1] = values[values.length - 1].append(value,);
  } else {
    while (values.length < index) values.push(Text.empty,);
    values.push(value,);
  }
}
function iterChanges(desc, f, individual,) {
  let inserted = desc.inserted;
  for (let posA = 0, posB = 0, i2 = 0; i2 < desc.sections.length;) {
    let len = desc.sections[i2++], ins = desc.sections[i2++];
    if (ins < 0) {
      posA += len;
      posB += len;
    } else {
      let endA = posA, endB = posB, text = Text.empty;
      for (;;) {
        endA += len;
        endB += ins;
        if (ins && inserted) text = text.append(inserted[i2 - 2 >> 1],);
        if (individual || i2 == desc.sections.length || desc.sections[i2 + 1] < 0) break;
        len = desc.sections[i2++];
        ins = desc.sections[i2++];
      }
      f(posA, endA, posB, endB, text,);
      posA = endA;
      posB = endB;
    }
  }
}
function mapSet(setA, setB, before, mkSet = false,) {
  let sections = [], insert2 = mkSet ? [] : null;
  let a = new SectionIter(setA,), b = new SectionIter(setB,);
  for (let inserted = -1;;) {
    if (a.ins == -1 && b.ins == -1) {
      let len = Math.min(a.len, b.len,);
      addSection(sections, len, -1,);
      a.forward(len,);
      b.forward(len,);
    } else if (b.ins >= 0 && (a.ins < 0 || inserted == a.i || a.off == 0 && (b.len < a.len || b.len == a.len && !before))) {
      let len1 = b.len;
      addSection(sections, b.ins, -1,);
      while (len1) {
        let piece = Math.min(a.len, len1,);
        if (a.ins >= 0 && inserted < a.i && a.len <= piece) {
          addSection(sections, 0, a.ins,);
          if (insert2) addInsert(insert2, sections, a.text,);
          inserted = a.i;
        }
        a.forward(piece,);
        len1 -= piece;
      }
      b.next();
    } else if (a.ins >= 0) {
      let len2 = 0, left = a.len;
      while (left) {
        if (b.ins == -1) {
          let piece1 = Math.min(left, b.len,);
          len2 += piece1;
          left -= piece1;
          b.forward(piece1,);
        } else if (b.ins == 0 && b.len < left) {
          left -= b.len;
          b.next();
        } else {
          break;
        }
      }
      addSection(sections, len2, inserted < a.i ? a.ins : 0,);
      if (insert2 && inserted < a.i) addInsert(insert2, sections, a.text,);
      inserted = a.i;
      a.forward(a.len - left,);
    } else if (a.done && b.done) {
      return insert2 ? ChangeSet.createSet(sections, insert2,) : ChangeDesc.create(sections,);
    } else {
      throw new Error('Mismatched change set lengths',);
    }
  }
}
function composeSets(setA, setB, mkSet = false,) {
  let sections = [];
  let insert2 = mkSet ? [] : null;
  let a = new SectionIter(setA,), b = new SectionIter(setB,);
  for (let open = false;;) {
    if (a.done && b.done) {
      return insert2 ? ChangeSet.createSet(sections, insert2,) : ChangeDesc.create(sections,);
    } else if (a.ins == 0) {
      addSection(sections, a.len, 0, open,);
      a.next();
    } else if (b.len == 0 && !b.done) {
      addSection(sections, 0, b.ins, open,);
      if (insert2) addInsert(insert2, sections, b.text,);
      b.next();
    } else if (a.done || b.done) {
      throw new Error('Mismatched change set lengths',);
    } else {
      let len = Math.min(a.len2, b.len,), sectionLen = sections.length;
      if (a.ins == -1) {
        let insB = b.ins == -1 ? -1 : b.off ? 0 : b.ins;
        addSection(sections, len, insB, open,);
        if (insert2 && insB) addInsert(insert2, sections, b.text,);
      } else if (b.ins == -1) {
        addSection(sections, a.off ? 0 : a.len, len, open,);
        if (insert2) addInsert(insert2, sections, a.textBit(len,),);
      } else {
        addSection(sections, a.off ? 0 : a.len, b.off ? 0 : b.ins, open,);
        if (insert2 && !b.off) addInsert(insert2, sections, b.text,);
      }
      open = (a.ins > len || b.ins >= 0 && b.len > len) && (open || sections.length > sectionLen);
      a.forward2(len,);
      b.forward(len,);
    }
  }
}
var SectionIter = class {
  next() {
    let { sections, } = this.set;
    if (this.i < sections.length) {
      this.len = sections[this.i++];
      this.ins = sections[this.i++];
    } else {
      this.len = 0;
      this.ins = -2;
    }
    this.off = 0;
  }
  get done() {
    return this.ins == -2;
  }
  get len2() {
    return this.ins < 0 ? this.len : this.ins;
  }
  get text() {
    let { inserted, } = this.set, index = this.i - 2 >> 1;
    return index >= inserted.length ? Text.empty : inserted[index];
  }
  textBit(len,) {
    let { inserted, } = this.set, index = this.i - 2 >> 1;
    return index >= inserted.length && !len ? Text.empty : inserted[index].slice(this.off, len == null ? void 0 : this.off + len,);
  }
  forward(len,) {
    if (len == this.len) this.next();
    else {
      this.len -= len;
      this.off += len;
    }
  }
  forward2(len,) {
    if (this.ins == -1) this.forward(len,);
    else if (len == this.ins) this.next();
    else {
      this.ins -= len;
      this.off += len;
    }
  }
  constructor(set,) {
    this.set = set;
    this.i = 0;
    this.next();
  }
};
var SelectionRange = class {
  /**
  The anchor of the range—the side that doesn't move when you
  extend it.
  */
  get anchor() {
    return this.flags & 16 ? this.to : this.from;
  }
  /**
  The head of the range, which is moved when the range is
  [extended](https://codemirror.net/6/docs/ref/#state.SelectionRange.extend).
  */
  get head() {
    return this.flags & 16 ? this.from : this.to;
  }
  /**
  True when `anchor` and `head` are at the same position.
  */
  get empty() {
    return this.from == this.to;
  }
  /**
  If this is a cursor that is explicitly associated with the
  character on one of its sides, this returns the side. -1 means
  the character before its position, 1 the character after, and 0
  means no association.
  */
  get assoc() {
    return this.flags & 4 ? -1 : this.flags & 8 ? 1 : 0;
  }
  /**
  The bidirectional text level associated with this cursor, if
  any.
  */
  get bidiLevel() {
    let level = this.flags & 3;
    return level == 3 ? null : level;
  }
  /**
  The goal column (stored vertical offset) associated with a
  cursor. This is used to preserve the vertical position when
  [moving](https://codemirror.net/6/docs/ref/#view.EditorView.moveVertically) across
  lines of different length.
  */
  get goalColumn() {
    let value = this.flags >> 5;
    return value == 33554431 ? void 0 : value;
  }
  /**
  Map this range through a change, producing a valid range in the
  updated document.
  */
  map(change, assoc = -1,) {
    let from, to;
    if (this.empty) {
      from = to = change.mapPos(this.from, assoc,);
    } else {
      from = change.mapPos(this.from, 1,);
      to = change.mapPos(this.to, -1,);
    }
    return from == this.from && to == this.to ? this : new SelectionRange(from, to, this.flags,);
  }
  /**
  Extend this range to cover at least `from` to `to`.
  */
  extend(from, to = from,) {
    if (from <= this.anchor && to >= this.anchor) return EditorSelection.range(from, to,);
    let head = Math.abs(from - this.anchor,) > Math.abs(to - this.anchor,) ? from : to;
    return EditorSelection.range(this.anchor, head,);
  }
  /**
  Compare this range to another range.
  */
  eq(other,) {
    return this.anchor == other.anchor && this.head == other.head;
  }
  /**
  Return a JSON-serializable object representing the range.
  */
  toJSON() {
    return { anchor: this.anchor, head: this.head, };
  }
  /**
  Convert a JSON representation of a range to a `SelectionRange`
  instance.
  */
  static fromJSON(json,) {
    if (!json || typeof json.anchor != 'number' || typeof json.head != 'number') {
      throw new RangeError('Invalid JSON representation for SelectionRange',);
    }
    return EditorSelection.range(json.anchor, json.head,);
  }
  /**
  @internal
  */
  static create(from, to, flags,) {
    return new SelectionRange(from, to, flags,);
  }
  constructor(from, to, flags,) {
    this.from = from;
    this.to = to;
    this.flags = flags;
  }
};
var EditorSelection = class {
  /**
  Map a selection through a change. Used to adjust the selection
  position for changes.
  */
  map(change, assoc = -1,) {
    if (change.empty) return this;
    return EditorSelection.create(this.ranges.map((r,) => r.map(change, assoc,)), this.mainIndex,);
  }
  /**
  Compare this selection to another selection.
  */
  eq(other,) {
    if (this.ranges.length != other.ranges.length || this.mainIndex != other.mainIndex) return false;
    for (let i2 = 0; i2 < this.ranges.length; i2++) if (!this.ranges[i2].eq(other.ranges[i2],)) return false;
    return true;
  }
  /**
  Get the primary selection range. Usually, you should make sure
  your code applies to _all_ ranges, by using methods like
  [`changeByRange`](https://codemirror.net/6/docs/ref/#state.EditorState.changeByRange).
  */
  get main() {
    return this.ranges[this.mainIndex];
  }
  /**
  Make sure the selection only has one range. Returns a selection
  holding only the main range from this selection.
  */
  asSingle() {
    return this.ranges.length == 1 ? this : new EditorSelection([this.main,], 0,);
  }
  /**
  Extend this selection with an extra range.
  */
  addRange(range, main = true,) {
    return EditorSelection.create([range,].concat(this.ranges,), main ? 0 : this.mainIndex + 1,);
  }
  /**
  Replace a given range with another range, and then normalize the
  selection to merge and sort ranges if necessary.
  */
  replaceRange(range, which = this.mainIndex,) {
    let ranges = this.ranges.slice();
    ranges[which] = range;
    return EditorSelection.create(ranges, this.mainIndex,);
  }
  /**
  Convert this selection to an object that can be serialized to
  JSON.
  */
  toJSON() {
    return { ranges: this.ranges.map((r,) => r.toJSON()), main: this.mainIndex, };
  }
  /**
  Create a selection from a JSON representation.
  */
  static fromJSON(json,) {
    if (!json || !Array.isArray(json.ranges,) || typeof json.main != 'number' || json.main >= json.ranges.length) {
      throw new RangeError('Invalid JSON representation for EditorSelection',);
    }
    return new EditorSelection(json.ranges.map((r,) => SelectionRange.fromJSON(r,)), json.main,);
  }
  /**
  Create a selection holding a single range.
  */
  static single(anchor, head = anchor,) {
    return new EditorSelection([EditorSelection.range(anchor, head,),], 0,);
  }
  /**
  Sort and merge the given set of ranges, creating a valid
  selection.
  */
  static create(ranges, mainIndex = 0,) {
    if (ranges.length == 0) throw new RangeError('A selection needs at least one range',);
    for (let pos = 0, i2 = 0; i2 < ranges.length; i2++) {
      let range = ranges[i2];
      if (range.empty ? range.from <= pos : range.from < pos) return EditorSelection.normalized(ranges.slice(), mainIndex,);
      pos = range.to;
    }
    return new EditorSelection(ranges, mainIndex,);
  }
  /**
  Create a cursor selection range at the given position. You can
  safely ignore the optional arguments in most situations.
  */
  static cursor(pos, assoc = 0, bidiLevel, goalColumn,) {
    return SelectionRange.create(
      pos,
      pos,
      (assoc == 0 ? 0 : assoc < 0 ? 4 : 8) | (bidiLevel == null ? 3 : Math.min(2, bidiLevel,)) |
        (goalColumn !== null && goalColumn !== void 0 ? goalColumn : 33554431) << 5,
    );
  }
  /**
  Create a selection range.
  */
  static range(anchor, head, goalColumn, bidiLevel,) {
    let flags = (goalColumn !== null && goalColumn !== void 0 ? goalColumn : 33554431) << 5 |
      (bidiLevel == null ? 3 : Math.min(2, bidiLevel,));
    return head < anchor
      ? SelectionRange.create(head, anchor, 16 | 8 | flags,)
      : SelectionRange.create(anchor, head, (head > anchor ? 4 : 0) | flags,);
  }
  /**
  @internal
  */
  static normalized(ranges, mainIndex = 0,) {
    let main = ranges[mainIndex];
    ranges.sort((a, b,) => a.from - b.from);
    mainIndex = ranges.indexOf(main,);
    for (let i2 = 1; i2 < ranges.length; i2++) {
      let range = ranges[i2], prev = ranges[i2 - 1];
      if (range.empty ? range.from <= prev.to : range.from < prev.to) {
        let from = prev.from, to = Math.max(range.to, prev.to,);
        if (i2 <= mainIndex) mainIndex--;
        ranges.splice(--i2, 2, range.anchor > range.head ? EditorSelection.range(to, from,) : EditorSelection.range(from, to,),);
      }
    }
    return new EditorSelection(ranges, mainIndex,);
  }
  constructor(ranges, mainIndex,) {
    this.ranges = ranges;
    this.mainIndex = mainIndex;
  }
};
function checkSelection(selection, docLength,) {
  for (let range of selection.ranges) if (range.to > docLength) throw new RangeError('Selection points outside of document',);
}
var nextID = 0;
var Facet = class {
  /**
  Define a new facet.
  */
  static define(config = {},) {
    return new Facet(
      config.combine || ((a,) => a),
      config.compareInput || ((a, b,) => a === b),
      config.compare || (!config.combine ? sameArray : (a, b,) => a === b),
      !!config.static,
      config.enables,
    );
  }
  /**
  Returns an extension that adds the given value to this facet.
  */
  of(value,) {
    return new FacetProvider([], this, 0, value,);
  }
  /**
  Create an extension that computes a value for the facet from a
  state. You must take care to declare the parts of the state that
  this value depends on, since your function is only called again
  for a new state when one of those parts changed.

  In cases where your value depends only on a single field, you'll
  want to use the [`from`](https://codemirror.net/6/docs/ref/#state.Facet.from) method instead.
  */
  compute(deps, get,) {
    if (this.isStatic) throw new Error('Can\'t compute a static facet',);
    return new FacetProvider(deps, this, 1, get,);
  }
  /**
  Create an extension that computes zero or more values for this
  facet from a state.
  */
  computeN(deps, get,) {
    if (this.isStatic) throw new Error('Can\'t compute a static facet',);
    return new FacetProvider(deps, this, 2, get,);
  }
  from(field, get,) {
    if (!get) get = (x,) => x;
    return this.compute([field,], (state,) => get(state.field(field,),),);
  }
  constructor(combine, compareInput, compare2, isStatic, enables,) {
    this.combine = combine;
    this.compareInput = compareInput;
    this.compare = compare2;
    this.isStatic = isStatic;
    this.id = nextID++;
    this.default = combine([],);
    this.extensions = typeof enables == 'function' ? enables(this,) : enables;
  }
};
function sameArray(a, b,) {
  return a == b || a.length == b.length && a.every((e, i2,) => e === b[i2]);
}
var FacetProvider = class {
  dynamicSlot(addresses,) {
    var _a2;
    let getter = this.value;
    let compare2 = this.facet.compareInput;
    let id2 = this.id, idx = addresses[id2] >> 1, multi = this.type == 2;
    let depDoc = false, depSel = false, depAddrs = [];
    for (let dep of this.dependencies) {
      if (dep == 'doc') depDoc = true;
      else if (dep == 'selection') depSel = true;
      else if ((((_a2 = addresses[dep.id]) !== null && _a2 !== void 0 ? _a2 : 1) & 1) == 0) depAddrs.push(addresses[dep.id],);
    }
    return {
      create(state,) {
        state.values[idx] = getter(state,);
        return 1;
      },
      update(state, tr,) {
        if (depDoc && tr.docChanged || depSel && (tr.docChanged || tr.selection) || ensureAll(state, depAddrs,)) {
          let newVal = getter(state,);
          if (multi ? !compareArray(newVal, state.values[idx], compare2,) : !compare2(newVal, state.values[idx],)) {
            state.values[idx] = newVal;
            return 1;
          }
        }
        return 0;
      },
      reconfigure: (state, oldState,) => {
        let newVal, oldAddr = oldState.config.address[id2];
        if (oldAddr != null) {
          let oldVal = getAddr(oldState, oldAddr,);
          if (
            this.dependencies.every((dep,) => {
              return dep instanceof Facet
                ? oldState.facet(dep,) === state.facet(dep,)
                : dep instanceof StateField
                ? oldState.field(dep, false,) == state.field(dep, false,)
                : true;
            },) || (multi ? compareArray(newVal = getter(state,), oldVal, compare2,) : compare2(newVal = getter(state,), oldVal,))
          ) {
            state.values[idx] = oldVal;
            return 0;
          }
        } else {
          newVal = getter(state,);
        }
        state.values[idx] = newVal;
        return 1;
      },
    };
  }
  constructor(dependencies, facet, type, value,) {
    this.dependencies = dependencies;
    this.facet = facet;
    this.type = type;
    this.value = value;
    this.id = nextID++;
  }
};
function compareArray(a, b, compare2,) {
  if (a.length != b.length) return false;
  for (let i2 = 0; i2 < a.length; i2++) if (!compare2(a[i2], b[i2],)) return false;
  return true;
}
function ensureAll(state, addrs,) {
  let changed = false;
  for (let addr of addrs) if (ensureAddr(state, addr,) & 1) changed = true;
  return changed;
}
function dynamicFacetSlot(addresses, facet, providers,) {
  let providerAddrs = providers.map((p,) => addresses[p.id]);
  let providerTypes = providers.map((p,) => p.type);
  let dynamic = providerAddrs.filter((p,) => !(p & 1));
  let idx = addresses[facet.id] >> 1;
  function get(state,) {
    let values = [];
    for (let i2 = 0; i2 < providerAddrs.length; i2++) {
      let value = getAddr(state, providerAddrs[i2],);
      if (providerTypes[i2] == 2) for (let val of value) values.push(val,);
      else values.push(value,);
    }
    return facet.combine(values,);
  }
  return {
    create(state,) {
      for (let addr of providerAddrs) ensureAddr(state, addr,);
      state.values[idx] = get(state,);
      return 1;
    },
    update(state, tr,) {
      if (!ensureAll(state, dynamic,)) return 0;
      let value = get(state,);
      if (facet.compare(value, state.values[idx],)) return 0;
      state.values[idx] = value;
      return 1;
    },
    reconfigure(state, oldState,) {
      let depChanged = ensureAll(state, providerAddrs,);
      let oldProviders = oldState.config.facets[facet.id], oldValue = oldState.facet(facet,);
      if (oldProviders && !depChanged && sameArray(providers, oldProviders,)) {
        state.values[idx] = oldValue;
        return 0;
      }
      let value = get(state,);
      if (facet.compare(value, oldValue,)) {
        state.values[idx] = oldValue;
        return 0;
      }
      state.values[idx] = value;
      return 1;
    },
  };
}
var initField = /* @__PURE__ */ Facet.define({ static: true, },);
var StateField = class {
  /**
  Define a state field.
  */
  static define(config,) {
    let field = new StateField(nextID++, config.create, config.update, config.compare || ((a, b,) => a === b), config,);
    if (config.provide) field.provides = config.provide(field,);
    return field;
  }
  create(state,) {
    let init = state.facet(initField,).find((i2,) => i2.field == this);
    return ((init === null || init === void 0 ? void 0 : init.create) || this.createF)(state,);
  }
  /**
  @internal
  */
  slot(addresses,) {
    let idx = addresses[this.id] >> 1;
    return {
      create: (state,) => {
        state.values[idx] = this.create(state,);
        return 1;
      },
      update: (state, tr,) => {
        let oldVal = state.values[idx];
        let value = this.updateF(oldVal, tr,);
        if (this.compareF(oldVal, value,)) return 0;
        state.values[idx] = value;
        return 1;
      },
      reconfigure: (state, oldState,) => {
        if (oldState.config.address[this.id] != null) {
          state.values[idx] = oldState.field(this,);
          return 0;
        }
        state.values[idx] = this.create(state,);
        return 1;
      },
    };
  }
  /**
  Returns an extension that enables this field and overrides the
  way it is initialized. Can be useful when you need to provide a
  non-default starting value for the field.
  */
  init(create,) {
    return [this, initField.of({ field: this, create, },),];
  }
  /**
  State field instances can be used as
  [`Extension`](https://codemirror.net/6/docs/ref/#state.Extension) values to enable the field in a
  given state.
  */
  get extension() {
    return this;
  }
  constructor(id2, createF, updateF, compareF, spec,) {
    this.id = id2;
    this.createF = createF;
    this.updateF = updateF;
    this.compareF = compareF;
    this.spec = spec;
    this.provides = void 0;
  }
};
var Prec_ = { lowest: 4, low: 3, default: 2, high: 1, highest: 0, };
function prec(value,) {
  return (ext,) => new PrecExtension(ext, value,);
}
var Prec = {
  /**
  The highest precedence level, for extensions that should end up
  near the start of the precedence ordering.
  */
  highest: /* @__PURE__ */ prec(Prec_.highest,),
  /**
  A higher-than-default precedence, for extensions that should
  come before those with default precedence.
  */
  high: /* @__PURE__ */ prec(Prec_.high,),
  /**
  The default precedence, which is also used for extensions
  without an explicit precedence.
  */
  default: /* @__PURE__ */ prec(Prec_.default,),
  /**
  A lower-than-default precedence.
  */
  low: /* @__PURE__ */ prec(Prec_.low,),
  /**
  The lowest precedence level. Meant for things that should end up
  near the end of the extension order.
  */
  lowest: /* @__PURE__ */ prec(Prec_.lowest,),
};
var PrecExtension = class {
  constructor(inner, prec2,) {
    this.inner = inner;
    this.prec = prec2;
  }
};
var Compartment = class {
  /**
  Create an instance of this compartment to add to your [state
  configuration](https://codemirror.net/6/docs/ref/#state.EditorStateConfig.extensions).
  */
  of(ext,) {
    return new CompartmentInstance(this, ext,);
  }
  /**
  Create an [effect](https://codemirror.net/6/docs/ref/#state.TransactionSpec.effects) that
  reconfigures this compartment.
  */
  reconfigure(content2,) {
    return Compartment.reconfigure.of({ compartment: this, extension: content2, },);
  }
  /**
  Get the current content of the compartment in the state, or
  `undefined` if it isn't present.
  */
  get(state,) {
    return state.config.compartments.get(this,);
  }
};
var CompartmentInstance = class {
  constructor(compartment, inner,) {
    this.compartment = compartment;
    this.inner = inner;
  }
};
var Configuration = class {
  staticFacet(facet,) {
    let addr = this.address[facet.id];
    return addr == null ? facet.default : this.staticValues[addr >> 1];
  }
  static resolve(base2, compartments, oldState,) {
    let fields = [];
    let facets = /* @__PURE__ */ Object.create(null,);
    let newCompartments = /* @__PURE__ */ new Map();
    for (let ext of flatten(base2, compartments, newCompartments,)) {
      if (ext instanceof StateField) fields.push(ext,);
      else (facets[ext.facet.id] || (facets[ext.facet.id] = [])).push(ext,);
    }
    let address = /* @__PURE__ */ Object.create(null,);
    let staticValues = [];
    let dynamicSlots = [];
    for (let field of fields) {
      address[field.id] = dynamicSlots.length << 1;
      dynamicSlots.push((a,) => field.slot(a,));
    }
    let oldFacets = oldState === null || oldState === void 0 ? void 0 : oldState.config.facets;
    for (let id2 in facets) {
      let providers = facets[id2], facet = providers[0].facet;
      let oldProviders = oldFacets && oldFacets[id2] || [];
      if (providers.every((p,) => p.type == 0)) {
        address[facet.id] = staticValues.length << 1 | 1;
        if (sameArray(oldProviders, providers,)) {
          staticValues.push(oldState.facet(facet,),);
        } else {
          let value = facet.combine(providers.map((p,) => p.value),);
          staticValues.push(oldState && facet.compare(value, oldState.facet(facet,),) ? oldState.facet(facet,) : value,);
        }
      } else {
        for (let p of providers) {
          if (p.type == 0) {
            address[p.id] = staticValues.length << 1 | 1;
            staticValues.push(p.value,);
          } else {
            address[p.id] = dynamicSlots.length << 1;
            dynamicSlots.push((a,) => p.dynamicSlot(a,));
          }
        }
        address[facet.id] = dynamicSlots.length << 1;
        dynamicSlots.push((a,) => dynamicFacetSlot(a, facet, providers,));
      }
    }
    let dynamic = dynamicSlots.map((f,) => f(address,));
    return new Configuration(base2, newCompartments, dynamic, address, staticValues, facets,);
  }
  constructor(base2, compartments, dynamicSlots, address, staticValues, facets,) {
    this.base = base2;
    this.compartments = compartments;
    this.dynamicSlots = dynamicSlots;
    this.address = address;
    this.staticValues = staticValues;
    this.facets = facets;
    this.statusTemplate = [];
    while (this.statusTemplate.length < dynamicSlots.length) this.statusTemplate.push(0,);
  }
};
function flatten(extension, compartments, newCompartments,) {
  let result = [[], [], [], [], [],];
  let seen = /* @__PURE__ */ new Map();
  function inner(ext, prec2,) {
    let known = seen.get(ext,);
    if (known != null) {
      if (known <= prec2) return;
      let found = result[known].indexOf(ext,);
      if (found > -1) result[known].splice(found, 1,);
      if (ext instanceof CompartmentInstance) newCompartments.delete(ext.compartment,);
    }
    seen.set(ext, prec2,);
    if (Array.isArray(ext,)) {
      for (let e of ext) inner(e, prec2,);
    } else if (ext instanceof CompartmentInstance) {
      if (newCompartments.has(ext.compartment,)) throw new RangeError(`Duplicate use of compartment in extensions`,);
      let content2 = compartments.get(ext.compartment,) || ext.inner;
      newCompartments.set(ext.compartment, content2,);
      inner(content2, prec2,);
    } else if (ext instanceof PrecExtension) {
      inner(ext.inner, ext.prec,);
    } else if (ext instanceof StateField) {
      result[prec2].push(ext,);
      if (ext.provides) inner(ext.provides, prec2,);
    } else if (ext instanceof FacetProvider) {
      result[prec2].push(ext,);
      if (ext.facet.extensions) inner(ext.facet.extensions, Prec_.default,);
    } else {
      let content1 = ext.extension;
      if (!content1) {
        throw new Error(
          `Unrecognized extension value in extension set (${ext}). This sometimes happens because multiple instances of @codemirror/state are loaded, breaking instanceof checks.`,
        );
      }
      inner(content1, prec2,);
    }
  }
  inner(extension, Prec_.default,);
  return result.reduce((a, b,) => a.concat(b,));
}
function ensureAddr(state, addr,) {
  if (addr & 1) return 2;
  let idx = addr >> 1;
  let status = state.status[idx];
  if (status == 4) throw new Error('Cyclic dependency between fields and/or facets',);
  if (status & 2) return status;
  state.status[idx] = 4;
  let changed = state.computeSlot(state, state.config.dynamicSlots[idx],);
  return state.status[idx] = 2 | changed;
}
function getAddr(state, addr,) {
  return addr & 1 ? state.config.staticValues[addr >> 1] : state.values[addr >> 1];
}
var languageData = /* @__PURE__ */ Facet.define();
var allowMultipleSelections = /* @__PURE__ */ Facet.define({ combine: (values,) => values.some((v,) => v), static: true, },);
var lineSeparator = /* @__PURE__ */ Facet.define({ combine: (values,) => values.length ? values[0] : void 0, static: true, },);
var changeFilter = /* @__PURE__ */ Facet.define();
var transactionFilter = /* @__PURE__ */ Facet.define();
var transactionExtender = /* @__PURE__ */ Facet.define();
var readOnly = /* @__PURE__ */ Facet.define({ combine: (values,) => values.length ? values[0] : false, },);
var Annotation = class {
  /**
  Define a new type of annotation.
  */
  static define() {
    return new AnnotationType();
  }
  /**
  @internal
  */
  constructor(type, value,) {
    this.type = type;
    this.value = value;
  }
};
var AnnotationType = class {
  /**
  Create an instance of this annotation.
  */
  of(value,) {
    return new Annotation(this, value,);
  }
};
var StateEffectType = class {
  /**
  Create a [state effect](https://codemirror.net/6/docs/ref/#state.StateEffect) instance of this
  type.
  */
  of(value,) {
    return new StateEffect(this, value,);
  }
  /**
  @internal
  */
  constructor(map,) {
    this.map = map;
  }
};
var StateEffect = class {
  /**
  Map this effect through a position mapping. Will return
  `undefined` when that ends up deleting the effect.
  */
  map(mapping,) {
    let mapped = this.type.map(this.value, mapping,);
    return mapped === void 0 ? void 0 : mapped == this.value ? this : new StateEffect(this.type, mapped,);
  }
  /**
  Tells you whether this effect object is of a given
  [type](https://codemirror.net/6/docs/ref/#state.StateEffectType).
  */
  is(type,) {
    return this.type == type;
  }
  /**
  Define a new effect type. The type parameter indicates the type
  of values that his effect holds. It should be a type that
  doesn't include `undefined`, since that is used in
  [mapping](https://codemirror.net/6/docs/ref/#state.StateEffect.map) to indicate that an effect is
  removed.
  */
  static define(spec = {},) {
    return new StateEffectType(spec.map || ((v,) => v),);
  }
  /**
  Map an array of effects through a change set.
  */
  static mapEffects(effects, mapping,) {
    if (!effects.length) return effects;
    let result = [];
    for (let effect of effects) {
      let mapped = effect.map(mapping,);
      if (mapped) result.push(mapped,);
    }
    return result;
  }
  /**
  @internal
  */
  constructor(type, value,) {
    this.type = type;
    this.value = value;
  }
};
StateEffect.reconfigure = /* @__PURE__ */ StateEffect.define();
StateEffect.appendConfig = /* @__PURE__ */ StateEffect.define();
var Transaction = class {
  /**
  @internal
  */
  static create(startState, changes, selection, effects, annotations, scrollIntoView2,) {
    return new Transaction(startState, changes, selection, effects, annotations, scrollIntoView2,);
  }
  /**
  The new document produced by the transaction. Contrary to
  [`.state`](https://codemirror.net/6/docs/ref/#state.Transaction.state)`.doc`, accessing this won't
  force the entire new state to be computed right away, so it is
  recommended that [transaction
  filters](https://codemirror.net/6/docs/ref/#state.EditorState^transactionFilter) use this getter
  when they need to look at the new document.
  */
  get newDoc() {
    return this._doc || (this._doc = this.changes.apply(this.startState.doc,));
  }
  /**
  The new selection produced by the transaction. If
  [`this.selection`](https://codemirror.net/6/docs/ref/#state.Transaction.selection) is undefined,
  this will [map](https://codemirror.net/6/docs/ref/#state.EditorSelection.map) the start state's
  current selection through the changes made by the transaction.
  */
  get newSelection() {
    return this.selection || this.startState.selection.map(this.changes,);
  }
  /**
  The new state created by the transaction. Computed on demand
  (but retained for subsequent access), so it is recommended not to
  access it in [transaction
  filters](https://codemirror.net/6/docs/ref/#state.EditorState^transactionFilter) when possible.
  */
  get state() {
    if (!this._state) this.startState.applyTransaction(this,);
    return this._state;
  }
  /**
  Get the value of the given annotation type, if any.
  */
  annotation(type,) {
    for (let ann of this.annotations) if (ann.type == type) return ann.value;
    return void 0;
  }
  /**
  Indicates whether the transaction changed the document.
  */
  get docChanged() {
    return !this.changes.empty;
  }
  /**
  Indicates whether this transaction reconfigures the state
  (through a [configuration compartment](https://codemirror.net/6/docs/ref/#state.Compartment) or
  with a top-level configuration
  [effect](https://codemirror.net/6/docs/ref/#state.StateEffect^reconfigure).
  */
  get reconfigured() {
    return this.startState.config != this.state.config;
  }
  /**
  Returns true if the transaction has a [user
  event](https://codemirror.net/6/docs/ref/#state.Transaction^userEvent) annotation that is equal to
  or more specific than `event`. For example, if the transaction
  has `"select.pointer"` as user event, `"select"` and
  `"select.pointer"` will match it.
  */
  isUserEvent(event,) {
    let e = this.annotation(Transaction.userEvent,);
    return !!(e && (e == event || e.length > event.length && e.slice(0, event.length,) == event && e[event.length] == '.'));
  }
  constructor(startState, changes, selection, effects, annotations, scrollIntoView2,) {
    this.startState = startState;
    this.changes = changes;
    this.selection = selection;
    this.effects = effects;
    this.annotations = annotations;
    this.scrollIntoView = scrollIntoView2;
    this._doc = null;
    this._state = null;
    if (selection) checkSelection(selection, changes.newLength,);
    if (!annotations.some((a,) => a.type == Transaction.time)) this.annotations = annotations.concat(Transaction.time.of(Date.now(),),);
  }
};
Transaction.time = /* @__PURE__ */ Annotation.define();
Transaction.userEvent = /* @__PURE__ */ Annotation.define();
Transaction.addToHistory = /* @__PURE__ */ Annotation.define();
Transaction.remote = /* @__PURE__ */ Annotation.define();
function joinRanges(a, b,) {
  let result = [];
  for (let iA = 0, iB = 0;;) {
    let from, to;
    if (iA < a.length && (iB == b.length || b[iB] >= a[iA])) {
      from = a[iA++];
      to = a[iA++];
    } else if (iB < b.length) {
      from = b[iB++];
      to = b[iB++];
    } else return result;
    if (!result.length || result[result.length - 1] < from) result.push(from, to,);
    else if (result[result.length - 1] < to) result[result.length - 1] = to;
  }
}
function mergeTransaction(a, b, sequential,) {
  var _a2;
  let mapForA, mapForB, changes;
  if (sequential) {
    mapForA = b.changes;
    mapForB = ChangeSet.empty(b.changes.length,);
    changes = a.changes.compose(b.changes,);
  } else {
    mapForA = b.changes.map(a.changes,);
    mapForB = a.changes.mapDesc(b.changes, true,);
    changes = a.changes.compose(mapForA,);
  }
  return {
    changes,
    selection: b.selection ? b.selection.map(mapForB,) : (_a2 = a.selection) === null || _a2 === void 0 ? void 0 : _a2.map(mapForA,),
    effects: StateEffect.mapEffects(a.effects, mapForA,).concat(StateEffect.mapEffects(b.effects, mapForB,),),
    annotations: a.annotations.length ? a.annotations.concat(b.annotations,) : b.annotations,
    scrollIntoView: a.scrollIntoView || b.scrollIntoView,
  };
}
function resolveTransactionInner(state, spec, docSize,) {
  let sel = spec.selection, annotations = asArray(spec.annotations,);
  if (spec.userEvent) annotations = annotations.concat(Transaction.userEvent.of(spec.userEvent,),);
  return {
    changes: spec.changes instanceof ChangeSet ? spec.changes : ChangeSet.of(spec.changes || [], docSize, state.facet(lineSeparator,),),
    selection: sel && (sel instanceof EditorSelection ? sel : EditorSelection.single(sel.anchor, sel.head,)),
    effects: asArray(spec.effects,),
    annotations,
    scrollIntoView: !!spec.scrollIntoView,
  };
}
function resolveTransaction(state, specs, filter,) {
  let s = resolveTransactionInner(state, specs.length ? specs[0] : {}, state.doc.length,);
  if (specs.length && specs[0].filter === false) filter = false;
  for (let i2 = 1; i2 < specs.length; i2++) {
    if (specs[i2].filter === false) filter = false;
    let seq = !!specs[i2].sequential;
    s = mergeTransaction(s, resolveTransactionInner(state, specs[i2], seq ? s.changes.newLength : state.doc.length,), seq,);
  }
  let tr = Transaction.create(state, s.changes, s.selection, s.effects, s.annotations, s.scrollIntoView,);
  return extendTransaction(filter ? filterTransaction(tr,) : tr,);
}
function filterTransaction(tr,) {
  let state = tr.startState;
  let result = true;
  for (let filter of state.facet(changeFilter,)) {
    let value = filter(tr,);
    if (value === false) {
      result = false;
      break;
    }
    if (Array.isArray(value,)) result = result === true ? value : joinRanges(result, value,);
  }
  if (result !== true) {
    let changes, back;
    if (result === false) {
      back = tr.changes.invertedDesc;
      changes = ChangeSet.empty(state.doc.length,);
    } else {
      let filtered = tr.changes.filter(result,);
      changes = filtered.changes;
      back = filtered.filtered.mapDesc(filtered.changes,).invertedDesc;
    }
    tr = Transaction.create(
      state,
      changes,
      tr.selection && tr.selection.map(back,),
      StateEffect.mapEffects(tr.effects, back,),
      tr.annotations,
      tr.scrollIntoView,
    );
  }
  let filters = state.facet(transactionFilter,);
  for (let i2 = filters.length - 1; i2 >= 0; i2--) {
    let filtered1 = filters[i2](tr,);
    if (filtered1 instanceof Transaction) tr = filtered1;
    else if (Array.isArray(filtered1,) && filtered1.length == 1 && filtered1[0] instanceof Transaction) tr = filtered1[0];
    else tr = resolveTransaction(state, asArray(filtered1,), false,);
  }
  return tr;
}
function extendTransaction(tr,) {
  let state = tr.startState, extenders = state.facet(transactionExtender,), spec = tr;
  for (let i2 = extenders.length - 1; i2 >= 0; i2--) {
    let extension = extenders[i2](tr,);
    if (extension && Object.keys(extension,).length) {
      spec = mergeTransaction(spec, resolveTransactionInner(state, extension, tr.changes.newLength,), true,);
    }
  }
  return spec == tr ? tr : Transaction.create(state, tr.changes, tr.selection, spec.effects, spec.annotations, spec.scrollIntoView,);
}
var none = [];
function asArray(value,) {
  return value == null ? none : Array.isArray(value,) ? value : [value,];
}
var CharCategory = /* @__PURE__ */ function (CharCategory2,) {
  CharCategory2[CharCategory2['Word'] = 0] = 'Word';
  CharCategory2[CharCategory2['Space'] = 1] = 'Space';
  CharCategory2[CharCategory2['Other'] = 2] = 'Other';
  return CharCategory2;
}(CharCategory || (CharCategory = {}),);
var nonASCIISingleCaseWordChar =
  /[\u00df\u0587\u0590-\u05f4\u0600-\u06ff\u3040-\u309f\u30a0-\u30ff\u3400-\u4db5\u4e00-\u9fcc\uac00-\ud7af]/;
var wordChar;
try {
  wordChar = /* @__PURE__ */ new RegExp('[\\p{Alphabetic}\\p{Number}_]', 'u',);
} catch (_) {
}
function hasWordChar(str,) {
  if (wordChar) return wordChar.test(str,);
  for (let i2 = 0; i2 < str.length; i2++) {
    let ch = str[i2];
    if (/\w/.test(ch,) || ch > '\x80' && (ch.toUpperCase() != ch.toLowerCase() || nonASCIISingleCaseWordChar.test(ch,))) return true;
  }
  return false;
}
function makeCategorizer(wordChars,) {
  return (char,) => {
    if (!/\S/.test(char,)) return CharCategory.Space;
    if (hasWordChar(char,)) return CharCategory.Word;
    for (let i2 = 0; i2 < wordChars.length; i2++) if (char.indexOf(wordChars[i2],) > -1) return CharCategory.Word;
    return CharCategory.Other;
  };
}
var EditorState = class {
  field(field, require2 = true,) {
    let addr = this.config.address[field.id];
    if (addr == null) {
      if (require2) throw new RangeError('Field is not present in this state',);
      return void 0;
    }
    ensureAddr(this, addr,);
    return getAddr(this, addr,);
  }
  /**
  Create a [transaction](https://codemirror.net/6/docs/ref/#state.Transaction) that updates this
  state. Any number of [transaction specs](https://codemirror.net/6/docs/ref/#state.TransactionSpec)
  can be passed. Unless
  [`sequential`](https://codemirror.net/6/docs/ref/#state.TransactionSpec.sequential) is set, the
  [changes](https://codemirror.net/6/docs/ref/#state.TransactionSpec.changes) (if any) of each spec
  are assumed to start in the _current_ document (not the document
  produced by previous specs), and its
  [selection](https://codemirror.net/6/docs/ref/#state.TransactionSpec.selection) and
  [effects](https://codemirror.net/6/docs/ref/#state.TransactionSpec.effects) are assumed to refer
  to the document created by its _own_ changes. The resulting
  transaction contains the combined effect of all the different
  specs. For [selection](https://codemirror.net/6/docs/ref/#state.TransactionSpec.selection), later
  specs take precedence over earlier ones.
  */
  update(...specs) {
    return resolveTransaction(this, specs, true,);
  }
  /**
  @internal
  */
  applyTransaction(tr,) {
    let conf = this.config, { base: base2, compartments, } = conf;
    for (let effect of tr.effects) {
      if (effect.is(Compartment.reconfigure,)) {
        if (conf) {
          compartments = /* @__PURE__ */ new Map();
          conf.compartments.forEach((val, key,) => compartments.set(key, val,));
          conf = null;
        }
        compartments.set(effect.value.compartment, effect.value.extension,);
      } else if (effect.is(StateEffect.reconfigure,)) {
        conf = null;
        base2 = effect.value;
      } else if (effect.is(StateEffect.appendConfig,)) {
        conf = null;
        base2 = asArray(base2,).concat(effect.value,);
      }
    }
    let startValues;
    if (!conf) {
      conf = Configuration.resolve(base2, compartments, this,);
      let intermediateState = new EditorState(
        conf,
        this.doc,
        this.selection,
        conf.dynamicSlots.map(() => null),
        (state, slot,) => slot.reconfigure(state, this,),
        null,
      );
      startValues = intermediateState.values;
    } else {
      startValues = tr.startState.values.slice();
    }
    new EditorState(conf, tr.newDoc, tr.newSelection, startValues, (state, slot,) => slot.update(state, tr,), tr,);
  }
  /**
  Create a [transaction spec](https://codemirror.net/6/docs/ref/#state.TransactionSpec) that
  replaces every selection range with the given content.
  */
  replaceSelection(text,) {
    if (typeof text == 'string') text = this.toText(text,);
    return this.changeByRange((range,) => ({
      changes: { from: range.from, to: range.to, insert: text, },
      range: EditorSelection.cursor(range.from + text.length,),
    }));
  }
  /**
  Create a set of changes and a new selection by running the given
  function for each range in the active selection. The function
  can return an optional set of changes (in the coordinate space
  of the start document), plus an updated range (in the coordinate
  space of the document produced by the call's own changes). This
  method will merge all the changes and ranges into a single
  changeset and selection, and return it as a [transaction
  spec](https://codemirror.net/6/docs/ref/#state.TransactionSpec), which can be passed to
  [`update`](https://codemirror.net/6/docs/ref/#state.EditorState.update).
  */
  changeByRange(f,) {
    let sel = this.selection;
    let result1 = f(sel.ranges[0],);
    let changes = this.changes(result1.changes,), ranges = [result1.range,];
    let effects = asArray(result1.effects,);
    for (let i2 = 1; i2 < sel.ranges.length; i2++) {
      let result = f(sel.ranges[i2],);
      let newChanges = this.changes(result.changes,), newMapped = newChanges.map(changes,);
      for (let j = 0; j < i2; j++) ranges[j] = ranges[j].map(newMapped,);
      let mapBy = changes.mapDesc(newChanges, true,);
      ranges.push(result.range.map(mapBy,),);
      changes = changes.compose(newMapped,);
      effects = StateEffect.mapEffects(effects, newMapped,).concat(StateEffect.mapEffects(asArray(result.effects,), mapBy,),);
    }
    return { changes, selection: EditorSelection.create(ranges, sel.mainIndex,), effects, };
  }
  /**
  Create a [change set](https://codemirror.net/6/docs/ref/#state.ChangeSet) from the given change
  description, taking the state's document length and line
  separator into account.
  */
  changes(spec = [],) {
    if (spec instanceof ChangeSet) return spec;
    return ChangeSet.of(spec, this.doc.length, this.facet(EditorState.lineSeparator,),);
  }
  /**
  Using the state's [line
  separator](https://codemirror.net/6/docs/ref/#state.EditorState^lineSeparator), create a
  [`Text`](https://codemirror.net/6/docs/ref/#state.Text) instance from the given string.
  */
  toText(string2,) {
    return Text.of(string2.split(this.facet(EditorState.lineSeparator,) || DefaultSplit,),);
  }
  /**
  Return the given range of the document as a string.
  */
  sliceDoc(from = 0, to = this.doc.length,) {
    return this.doc.sliceString(from, to, this.lineBreak,);
  }
  /**
  Get the value of a state [facet](https://codemirror.net/6/docs/ref/#state.Facet).
  */
  facet(facet,) {
    let addr = this.config.address[facet.id];
    if (addr == null) return facet.default;
    ensureAddr(this, addr,);
    return getAddr(this, addr,);
  }
  /**
  Convert this state to a JSON-serializable object. When custom
  fields should be serialized, you can pass them in as an object
  mapping property names (in the resulting object, which should
  not use `doc` or `selection`) to fields.
  */
  toJSON(fields,) {
    let result = { doc: this.sliceDoc(), selection: this.selection.toJSON(), };
    if (fields) {
      for (let prop in fields) {
        let value = fields[prop];
        if (value instanceof StateField && this.config.address[value.id] != null) {
          result[prop] = value.spec.toJSON(this.field(fields[prop],), this,);
        }
      }
    }
    return result;
  }
  /**
  Deserialize a state from its JSON representation. When custom
  fields should be deserialized, pass the same object you passed
  to [`toJSON`](https://codemirror.net/6/docs/ref/#state.EditorState.toJSON) when serializing as
  third argument.
  */
  static fromJSON(json, config = {}, fields,) {
    if (!json || typeof json.doc != 'string') throw new RangeError('Invalid JSON representation for EditorState',);
    let fieldInit = [];
    if (fields) {
      for (let prop in fields) {
        if (Object.prototype.hasOwnProperty.call(json, prop,)) {
          let field = fields[prop], value = json[prop];
          fieldInit.push(field.init((state,) => field.spec.fromJSON(value, state,)),);
        }
      }
    }
    return EditorState.create({
      doc: json.doc,
      selection: EditorSelection.fromJSON(json.selection,),
      extensions: config.extensions ? fieldInit.concat([config.extensions,],) : fieldInit,
    },);
  }
  /**
  Create a new state. You'll usually only need this when
  initializing an editor—updated states are created by applying
  transactions.
  */
  static create(config = {},) {
    let configuration = Configuration.resolve(config.extensions || [], /* @__PURE__ */ new Map(),);
    let doc2 = config.doc instanceof Text
      ? config.doc
      : Text.of((config.doc || '').split(configuration.staticFacet(EditorState.lineSeparator,) || DefaultSplit,),);
    let selection = !config.selection
      ? EditorSelection.single(0,)
      : config.selection instanceof EditorSelection
      ? config.selection
      : EditorSelection.single(config.selection.anchor, config.selection.head,);
    checkSelection(selection, doc2.length,);
    if (!configuration.staticFacet(allowMultipleSelections,)) selection = selection.asSingle();
    return new EditorState(
      configuration,
      doc2,
      selection,
      configuration.dynamicSlots.map(() => null),
      (state, slot,) => slot.create(state,),
      null,
    );
  }
  /**
  The size (in columns) of a tab in the document, determined by
  the [`tabSize`](https://codemirror.net/6/docs/ref/#state.EditorState^tabSize) facet.
  */
  get tabSize() {
    return this.facet(EditorState.tabSize,);
  }
  /**
  Get the proper [line-break](https://codemirror.net/6/docs/ref/#state.EditorState^lineSeparator)
  string for this state.
  */
  get lineBreak() {
    return this.facet(EditorState.lineSeparator,) || '\n';
  }
  /**
  Returns true when the editor is
  [configured](https://codemirror.net/6/docs/ref/#state.EditorState^readOnly) to be read-only.
  */
  get readOnly() {
    return this.facet(readOnly,);
  }
  /**
  Look up a translation for the given phrase (via the
  [`phrases`](https://codemirror.net/6/docs/ref/#state.EditorState^phrases) facet), or return the
  original string if no translation is found.

  If additional arguments are passed, they will be inserted in
  place of markers like `$1` (for the first value) and `$2`, etc.
  A single `$` is equivalent to `$1`, and `$$` will produce a
  literal dollar sign.
  */
  phrase(phrase, ...insert2) {
    for (let map of this.facet(EditorState.phrases,)) {
      if (Object.prototype.hasOwnProperty.call(map, phrase,)) {
        phrase = map[phrase];
        break;
      }
    }
    if (insert2.length) {
      phrase = phrase.replace(/\$(\$|\d*)/g, (m, i2,) => {
        if (i2 == '$') return '$';
        let n = +(i2 || 1);
        return !n || n > insert2.length ? m : insert2[n - 1];
      },);
    }
    return phrase;
  }
  /**
  Find the values for a given language data field, provided by the
  the [`languageData`](https://codemirror.net/6/docs/ref/#state.EditorState^languageData) facet.

  Examples of language data fields are...

  - [`"commentTokens"`](https://codemirror.net/6/docs/ref/#commands.CommentTokens) for specifying
    comment syntax.
  - [`"autocomplete"`](https://codemirror.net/6/docs/ref/#autocomplete.autocompletion^config.override)
    for providing language-specific completion sources.
  - [`"wordChars"`](https://codemirror.net/6/docs/ref/#state.EditorState.charCategorizer) for adding
    characters that should be considered part of words in this
    language.
  - [`"closeBrackets"`](https://codemirror.net/6/docs/ref/#autocomplete.CloseBracketConfig) controls
    bracket closing behavior.
  */
  languageDataAt(name2, pos, side = -1,) {
    let values = [];
    for (let provider of this.facet(languageData,)) {
      for (let result of provider(this, pos, side,)) {
        if (Object.prototype.hasOwnProperty.call(result, name2,)) values.push(result[name2],);
      }
    }
    return values;
  }
  /**
  Return a function that can categorize strings (expected to
  represent a single [grapheme cluster](https://codemirror.net/6/docs/ref/#state.findClusterBreak))
  into one of:

   - Word (contains an alphanumeric character or a character
     explicitly listed in the local language's `"wordChars"`
     language data, which should be a string)
   - Space (contains only whitespace)
   - Other (anything else)
  */
  charCategorizer(at,) {
    return makeCategorizer(this.languageDataAt('wordChars', at,).join('',),);
  }
  /**
  Find the word at the given position, meaning the range
  containing all [word](https://codemirror.net/6/docs/ref/#state.CharCategory.Word) characters
  around it. If no word characters are adjacent to the position,
  this returns null.
  */
  wordAt(pos,) {
    let { text, from, length, } = this.doc.lineAt(pos,);
    let cat = this.charCategorizer(pos,);
    let start = pos - from, end = pos - from;
    while (start > 0) {
      let prev = findClusterBreak(text, start, false,);
      if (cat(text.slice(prev, start,),) != CharCategory.Word) break;
      start = prev;
    }
    while (end < length) {
      let next = findClusterBreak(text, end,);
      if (cat(text.slice(end, next,),) != CharCategory.Word) break;
      end = next;
    }
    return start == end ? null : EditorSelection.range(start + from, end + from,);
  }
  constructor(config, doc2, selection, values, computeSlot, tr,) {
    this.config = config;
    this.doc = doc2;
    this.selection = selection;
    this.values = values;
    this.status = config.statusTemplate.slice();
    this.computeSlot = computeSlot;
    if (tr) tr._state = this;
    for (let i2 = 0; i2 < this.config.dynamicSlots.length; i2++) ensureAddr(this, i2 << 1,);
    this.computeSlot = null;
  }
};
EditorState.allowMultipleSelections = allowMultipleSelections;
EditorState.tabSize = /* @__PURE__ */ Facet.define({ combine: (values,) => values.length ? values[0] : 4, },);
EditorState.lineSeparator = lineSeparator;
EditorState.readOnly = readOnly;
EditorState.phrases = /* @__PURE__ */ Facet.define({
  compare(a, b,) {
    let kA = Object.keys(a,), kB = Object.keys(b,);
    return kA.length == kB.length && kA.every((k,) => a[k] == b[k]);
  },
},);
EditorState.languageData = languageData;
EditorState.changeFilter = changeFilter;
EditorState.transactionFilter = transactionFilter;
EditorState.transactionExtender = transactionExtender;
Compartment.reconfigure = /* @__PURE__ */ StateEffect.define();
function combineConfig(configs, defaults, combine = {},) {
  let result = {};
  for (let config of configs) {
    for (let key of Object.keys(config,)) {
      let value = config[key], current = result[key];
      if (current === void 0) result[key] = value;
      else if (current === value || value === void 0);
      else if (Object.hasOwnProperty.call(combine, key,)) result[key] = combine[key](current, value,);
      else throw new Error('Config merge conflict for field ' + key,);
    }
  }
  for (let key1 in defaults) if (result[key1] === void 0) result[key1] = defaults[key1];
  return result;
}
var RangeValue = class {
  /**
  Compare this value with another value. Used when comparing
  rangesets. The default implementation compares by identity.
  Unless you are only creating a fixed number of unique instances
  of your value type, it is a good idea to implement this
  properly.
  */
  eq(other,) {
    return this == other;
  }
  /**
  Create a [range](https://codemirror.net/6/docs/ref/#state.Range) with this value.
  */
  range(from, to = from,) {
    return Range.create(from, to, this,);
  }
};
RangeValue.prototype.startSide = RangeValue.prototype.endSide = 0;
RangeValue.prototype.point = false;
RangeValue.prototype.mapMode = MapMode.TrackDel;
var Range = class {
  /**
  @internal
  */
  static create(from, to, value,) {
    return new Range(from, to, value,);
  }
  constructor(from, to, value,) {
    this.from = from;
    this.to = to;
    this.value = value;
  }
};
function cmpRange(a, b,) {
  return a.from - b.from || a.value.startSide - b.value.startSide;
}
var Chunk = class {
  get length() {
    return this.to[this.to.length - 1];
  }
  // Find the index of the given position and side. Use the ranges'
  // `from` pos when `end == false`, `to` when `end == true`.
  findIndex(pos, side, end, startAt = 0,) {
    let arr = end ? this.to : this.from;
    for (let lo = startAt, hi = arr.length;;) {
      if (lo == hi) return lo;
      let mid = lo + hi >> 1;
      let diff = arr[mid] - pos || (end ? this.value[mid].endSide : this.value[mid].startSide) - side;
      if (mid == lo) return diff >= 0 ? lo : hi;
      if (diff >= 0) hi = mid;
      else lo = mid + 1;
    }
  }
  between(offset, from, to, f,) {
    for (let i2 = this.findIndex(from, -1e9, true,), e = this.findIndex(to, 1e9, false, i2,); i2 < e; i2++) {
      if (f(this.from[i2] + offset, this.to[i2] + offset, this.value[i2],) === false) return false;
    }
  }
  map(offset, changes,) {
    let value = [], from = [], to = [], newPos = -1, maxPoint = -1;
    for (let i2 = 0; i2 < this.value.length; i2++) {
      let val = this.value[i2], curFrom = this.from[i2] + offset, curTo = this.to[i2] + offset, newFrom, newTo;
      if (curFrom == curTo) {
        let mapped = changes.mapPos(curFrom, val.startSide, val.mapMode,);
        if (mapped == null) continue;
        newFrom = newTo = mapped;
        if (val.startSide != val.endSide) {
          newTo = changes.mapPos(curFrom, val.endSide,);
          if (newTo < newFrom) continue;
        }
      } else {
        newFrom = changes.mapPos(curFrom, val.startSide,);
        newTo = changes.mapPos(curTo, val.endSide,);
        if (newFrom > newTo || newFrom == newTo && val.startSide > 0 && val.endSide <= 0) continue;
      }
      if ((newTo - newFrom || val.endSide - val.startSide) < 0) continue;
      if (newPos < 0) newPos = newFrom;
      if (val.point) maxPoint = Math.max(maxPoint, newTo - newFrom,);
      value.push(val,);
      from.push(newFrom - newPos,);
      to.push(newTo - newPos,);
    }
    return { mapped: value.length ? new Chunk(from, to, value, maxPoint,) : null, pos: newPos, };
  }
  constructor(from, to, value, maxPoint,) {
    this.from = from;
    this.to = to;
    this.value = value;
    this.maxPoint = maxPoint;
  }
};
var RangeSet = class {
  /**
  @internal
  */
  static create(chunkPos, chunk, nextLayer, maxPoint,) {
    return new RangeSet(chunkPos, chunk, nextLayer, maxPoint,);
  }
  /**
  @internal
  */
  get length() {
    let last = this.chunk.length - 1;
    return last < 0 ? 0 : Math.max(this.chunkEnd(last,), this.nextLayer.length,);
  }
  /**
  The number of ranges in the set.
  */
  get size() {
    if (this.isEmpty) return 0;
    let size = this.nextLayer.size;
    for (let chunk of this.chunk) size += chunk.value.length;
    return size;
  }
  /**
  @internal
  */
  chunkEnd(index,) {
    return this.chunkPos[index] + this.chunk[index].length;
  }
  /**
  Update the range set, optionally adding new ranges or filtering
  out existing ones.

  (Note: The type parameter is just there as a kludge to work
  around TypeScript variance issues that prevented `RangeSet<X>`
  from being a subtype of `RangeSet<Y>` when `X` is a subtype of
  `Y`.)
  */
  update(updateSpec,) {
    let { add = [], sort = false, filterFrom = 0, filterTo = this.length, } = updateSpec;
    let filter = updateSpec.filter;
    if (add.length == 0 && !filter) return this;
    if (sort) add = add.slice().sort(cmpRange,);
    if (this.isEmpty) return add.length ? RangeSet.of(add,) : this;
    let cur = new LayerCursor(this, null, -1,).goto(0,), i2 = 0, spill = [];
    let builder = new RangeSetBuilder();
    while (cur.value || i2 < add.length) {
      if (i2 < add.length && (cur.from - add[i2].from || cur.startSide - add[i2].value.startSide) >= 0) {
        let range = add[i2++];
        if (!builder.addInner(range.from, range.to, range.value,)) spill.push(range,);
      } else if (
        cur.rangeIndex == 1 && cur.chunkIndex < this.chunk.length && (i2 == add.length || this.chunkEnd(cur.chunkIndex,) < add[i2].from) &&
        (!filter || filterFrom > this.chunkEnd(cur.chunkIndex,) || filterTo < this.chunkPos[cur.chunkIndex]) &&
        builder.addChunk(this.chunkPos[cur.chunkIndex], this.chunk[cur.chunkIndex],)
      ) {
        cur.nextChunk();
      } else {
        if (!filter || filterFrom > cur.to || filterTo < cur.from || filter(cur.from, cur.to, cur.value,)) {
          if (!builder.addInner(cur.from, cur.to, cur.value,)) spill.push(Range.create(cur.from, cur.to, cur.value,),);
        }
        cur.next();
      }
    }
    return builder.finishInner(
      this.nextLayer.isEmpty && !spill.length ? RangeSet.empty : this.nextLayer.update({ add: spill, filter, filterFrom, filterTo, },),
    );
  }
  /**
  Map this range set through a set of changes, return the new set.
  */
  map(changes,) {
    if (changes.empty || this.isEmpty) return this;
    let chunks = [], chunkPos = [], maxPoint = -1;
    for (let i2 = 0; i2 < this.chunk.length; i2++) {
      let start = this.chunkPos[i2], chunk = this.chunk[i2];
      let touch = changes.touchesRange(start, start + chunk.length,);
      if (touch === false) {
        maxPoint = Math.max(maxPoint, chunk.maxPoint,);
        chunks.push(chunk,);
        chunkPos.push(changes.mapPos(start,),);
      } else if (touch === true) {
        let { mapped, pos, } = chunk.map(start, changes,);
        if (mapped) {
          maxPoint = Math.max(maxPoint, mapped.maxPoint,);
          chunks.push(mapped,);
          chunkPos.push(pos,);
        }
      }
    }
    let next = this.nextLayer.map(changes,);
    return chunks.length == 0 ? next : new RangeSet(chunkPos, chunks, next || RangeSet.empty, maxPoint,);
  }
  /**
  Iterate over the ranges that touch the region `from` to `to`,
  calling `f` for each. There is no guarantee that the ranges will
  be reported in any specific order. When the callback returns
  `false`, iteration stops.
  */
  between(from, to, f,) {
    if (this.isEmpty) return;
    for (let i2 = 0; i2 < this.chunk.length; i2++) {
      let start = this.chunkPos[i2], chunk = this.chunk[i2];
      if (to >= start && from <= start + chunk.length && chunk.between(start, from - start, to - start, f,) === false) return;
    }
    this.nextLayer.between(from, to, f,);
  }
  /**
  Iterate over the ranges in this set, in order, including all
  ranges that end at or after `from`.
  */
  iter(from = 0,) {
    return HeapCursor.from([this,],).goto(from,);
  }
  /**
  @internal
  */
  get isEmpty() {
    return this.nextLayer == this;
  }
  /**
  Iterate over the ranges in a collection of sets, in order,
  starting from `from`.
  */
  static iter(sets, from = 0,) {
    return HeapCursor.from(sets,).goto(from,);
  }
  /**
  Iterate over two groups of sets, calling methods on `comparator`
  to notify it of possible differences.
  */
  static compare(oldSets, newSets, textDiff, comparator, minPointSize = -1,) {
    let a = oldSets.filter((set,) => set.maxPoint > 0 || !set.isEmpty && set.maxPoint >= minPointSize);
    let b = newSets.filter((set,) => set.maxPoint > 0 || !set.isEmpty && set.maxPoint >= minPointSize);
    let sharedChunks = findSharedChunks(a, b, textDiff,);
    let sideA = new SpanCursor(a, sharedChunks, minPointSize,);
    let sideB = new SpanCursor(b, sharedChunks, minPointSize,);
    textDiff.iterGaps((fromA, fromB, length,) => compare(sideA, fromA, sideB, fromB, length, comparator,));
    if (textDiff.empty && textDiff.length == 0) compare(sideA, 0, sideB, 0, 0, comparator,);
  }
  /**
  Compare the contents of two groups of range sets, returning true
  if they are equivalent in the given range.
  */
  static eq(oldSets, newSets, from = 0, to,) {
    if (to == null) to = 1e9 - 1;
    let a = oldSets.filter((set,) => !set.isEmpty && newSets.indexOf(set,) < 0);
    let b = newSets.filter((set,) => !set.isEmpty && oldSets.indexOf(set,) < 0);
    if (a.length != b.length) return false;
    if (!a.length) return true;
    let sharedChunks = findSharedChunks(a, b,);
    let sideA = new SpanCursor(a, sharedChunks, 0,).goto(from,), sideB = new SpanCursor(b, sharedChunks, 0,).goto(from,);
    for (;;) {
      if (
        sideA.to != sideB.to || !sameValues(sideA.active, sideB.active,) || sideA.point && (!sideB.point || !sideA.point.eq(sideB.point,))
      ) return false;
      if (sideA.to > to) return true;
      sideA.next();
      sideB.next();
    }
  }
  /**
  Iterate over a group of range sets at the same time, notifying
  the iterator about the ranges covering every given piece of
  content. Returns the open count (see
  [`SpanIterator.span`](https://codemirror.net/6/docs/ref/#state.SpanIterator.span)) at the end
  of the iteration.
  */
  static spans(sets, from, to, iterator, minPointSize = -1,) {
    let cursor = new SpanCursor(sets, null, minPointSize,).goto(from,), pos = from;
    let openRanges = cursor.openStart;
    for (;;) {
      let curTo = Math.min(cursor.to, to,);
      if (cursor.point) {
        let active = cursor.activeForPoint(cursor.to,);
        let openCount = cursor.pointFrom < from ? active.length + 1 : Math.min(active.length, openRanges,);
        iterator.point(pos, curTo, cursor.point, active, openCount, cursor.pointRank,);
        openRanges = Math.min(cursor.openEnd(curTo,), active.length,);
      } else if (curTo > pos) {
        iterator.span(pos, curTo, cursor.active, openRanges,);
        openRanges = cursor.openEnd(curTo,);
      }
      if (cursor.to > to) return openRanges + (cursor.point && cursor.to > to ? 1 : 0);
      pos = cursor.to;
      cursor.next();
    }
  }
  /**
  Create a range set for the given range or array of ranges. By
  default, this expects the ranges to be _sorted_ (by start
  position and, if two start at the same position,
  `value.startSide`). You can pass `true` as second argument to
  cause the method to sort them.
  */
  static of(ranges, sort = false,) {
    let build = new RangeSetBuilder();
    for (let range of ranges instanceof Range ? [ranges,] : sort ? lazySort(ranges,) : ranges) {
      build.add(range.from, range.to, range.value,);
    }
    return build.finish();
  }
  constructor(chunkPos, chunk, nextLayer, maxPoint,) {
    this.chunkPos = chunkPos;
    this.chunk = chunk;
    this.nextLayer = nextLayer;
    this.maxPoint = maxPoint;
  }
};
RangeSet.empty = /* @__PURE__ */ new RangeSet([], [], null, -1,);
function lazySort(ranges,) {
  if (ranges.length > 1) {
    for (let prev = ranges[0], i2 = 1; i2 < ranges.length; i2++) {
      let cur = ranges[i2];
      if (cmpRange(prev, cur,) > 0) return ranges.slice().sort(cmpRange,);
      prev = cur;
    }
  }
  return ranges;
}
RangeSet.empty.nextLayer = RangeSet.empty;
var RangeSetBuilder = class {
  finishChunk(newArrays,) {
    this.chunks.push(new Chunk(this.from, this.to, this.value, this.maxPoint,),);
    this.chunkPos.push(this.chunkStart,);
    this.chunkStart = -1;
    this.setMaxPoint = Math.max(this.setMaxPoint, this.maxPoint,);
    this.maxPoint = -1;
    if (newArrays) {
      this.from = [];
      this.to = [];
      this.value = [];
    }
  }
  /**
  Add a range. Ranges should be added in sorted (by `from` and
  `value.startSide`) order.
  */
  add(from, to, value,) {
    if (!this.addInner(from, to, value,)) (this.nextLayer || (this.nextLayer = new RangeSetBuilder())).add(from, to, value,);
  }
  /**
  @internal
  */
  addInner(from, to, value,) {
    let diff = from - this.lastTo || value.startSide - this.last.endSide;
    if (diff <= 0 && (from - this.lastFrom || value.startSide - this.last.startSide) < 0) {
      throw new Error('Ranges must be added sorted by `from` position and `startSide`',);
    }
    if (diff < 0) return false;
    if (this.from.length == 250) this.finishChunk(true,);
    if (this.chunkStart < 0) this.chunkStart = from;
    this.from.push(from - this.chunkStart,);
    this.to.push(to - this.chunkStart,);
    this.last = value;
    this.lastFrom = from;
    this.lastTo = to;
    this.value.push(value,);
    if (value.point) this.maxPoint = Math.max(this.maxPoint, to - from,);
    return true;
  }
  /**
  @internal
  */
  addChunk(from, chunk,) {
    if ((from - this.lastTo || chunk.value[0].startSide - this.last.endSide) < 0) return false;
    if (this.from.length) this.finishChunk(true,);
    this.setMaxPoint = Math.max(this.setMaxPoint, chunk.maxPoint,);
    this.chunks.push(chunk,);
    this.chunkPos.push(from,);
    let last = chunk.value.length - 1;
    this.last = chunk.value[last];
    this.lastFrom = chunk.from[last] + from;
    this.lastTo = chunk.to[last] + from;
    return true;
  }
  /**
  Finish the range set. Returns the new set. The builder can't be
  used anymore after this has been called.
  */
  finish() {
    return this.finishInner(RangeSet.empty,);
  }
  /**
  @internal
  */
  finishInner(next,) {
    if (this.from.length) this.finishChunk(false,);
    if (this.chunks.length == 0) return next;
    let result = RangeSet.create(this.chunkPos, this.chunks, this.nextLayer ? this.nextLayer.finishInner(next,) : next, this.setMaxPoint,);
    this.from = null;
    return result;
  }
  /**
  Create an empty builder.
  */
  constructor() {
    this.chunks = [];
    this.chunkPos = [];
    this.chunkStart = -1;
    this.last = null;
    this.lastFrom = -1e9;
    this.lastTo = -1e9;
    this.from = [];
    this.to = [];
    this.value = [];
    this.maxPoint = -1;
    this.setMaxPoint = -1;
    this.nextLayer = null;
  }
};
function findSharedChunks(a, b, textDiff,) {
  let inA = /* @__PURE__ */ new Map();
  for (let set of a) {
    for (let i2 = 0; i2 < set.chunk.length; i2++) if (set.chunk[i2].maxPoint <= 0) inA.set(set.chunk[i2], set.chunkPos[i2],);
  }
  let shared = /* @__PURE__ */ new Set();
  for (let set1 of b) {
    for (let i1 = 0; i1 < set1.chunk.length; i1++) {
      let known = inA.get(set1.chunk[i1],);
      if (
        known != null && (textDiff ? textDiff.mapPos(known,) : known) == set1.chunkPos[i1] &&
        !(textDiff === null || textDiff === void 0 ? void 0 : textDiff.touchesRange(known, known + set1.chunk[i1].length,))
      ) shared.add(set1.chunk[i1],);
    }
  }
  return shared;
}
var LayerCursor = class {
  get startSide() {
    return this.value ? this.value.startSide : 0;
  }
  get endSide() {
    return this.value ? this.value.endSide : 0;
  }
  goto(pos, side = -1e9,) {
    this.chunkIndex = this.rangeIndex = 0;
    this.gotoInner(pos, side, false,);
    return this;
  }
  gotoInner(pos, side, forward,) {
    while (this.chunkIndex < this.layer.chunk.length) {
      let next = this.layer.chunk[this.chunkIndex];
      if (!(this.skip && this.skip.has(next,) || this.layer.chunkEnd(this.chunkIndex,) < pos || next.maxPoint < this.minPoint)) break;
      this.chunkIndex++;
      forward = false;
    }
    if (this.chunkIndex < this.layer.chunk.length) {
      let rangeIndex = this.layer.chunk[this.chunkIndex].findIndex(pos - this.layer.chunkPos[this.chunkIndex], side, true,);
      if (!forward || this.rangeIndex < rangeIndex) this.setRangeIndex(rangeIndex,);
    }
    this.next();
  }
  forward(pos, side,) {
    if ((this.to - pos || this.endSide - side) < 0) this.gotoInner(pos, side, true,);
  }
  next() {
    for (;;) {
      if (this.chunkIndex == this.layer.chunk.length) {
        this.from = this.to = 1e9;
        this.value = null;
        break;
      } else {
        let chunkPos = this.layer.chunkPos[this.chunkIndex], chunk = this.layer.chunk[this.chunkIndex];
        let from = chunkPos + chunk.from[this.rangeIndex];
        this.from = from;
        this.to = chunkPos + chunk.to[this.rangeIndex];
        this.value = chunk.value[this.rangeIndex];
        this.setRangeIndex(this.rangeIndex + 1,);
        if (this.minPoint < 0 || this.value.point && this.to - this.from >= this.minPoint) break;
      }
    }
  }
  setRangeIndex(index,) {
    if (index == this.layer.chunk[this.chunkIndex].value.length) {
      this.chunkIndex++;
      if (this.skip) {
        while (this.chunkIndex < this.layer.chunk.length && this.skip.has(this.layer.chunk[this.chunkIndex],)) this.chunkIndex++;
      }
      this.rangeIndex = 0;
    } else {
      this.rangeIndex = index;
    }
  }
  nextChunk() {
    this.chunkIndex++;
    this.rangeIndex = 0;
    this.next();
  }
  compare(other,) {
    return this.from - other.from || this.startSide - other.startSide || this.rank - other.rank || this.to - other.to ||
      this.endSide - other.endSide;
  }
  constructor(layer, skip, minPoint, rank = 0,) {
    this.layer = layer;
    this.skip = skip;
    this.minPoint = minPoint;
    this.rank = rank;
  }
};
var HeapCursor = class {
  static from(sets, skip = null, minPoint = -1,) {
    let heap = [];
    for (let i2 = 0; i2 < sets.length; i2++) {
      for (let cur = sets[i2]; !cur.isEmpty; cur = cur.nextLayer) {
        if (cur.maxPoint >= minPoint) heap.push(new LayerCursor(cur, skip, minPoint, i2,),);
      }
    }
    return heap.length == 1 ? heap[0] : new HeapCursor(heap,);
  }
  get startSide() {
    return this.value ? this.value.startSide : 0;
  }
  goto(pos, side = -1e9,) {
    for (let cur of this.heap) cur.goto(pos, side,);
    for (let i2 = this.heap.length >> 1; i2 >= 0; i2--) heapBubble(this.heap, i2,);
    this.next();
    return this;
  }
  forward(pos, side,) {
    for (let cur of this.heap) cur.forward(pos, side,);
    for (let i2 = this.heap.length >> 1; i2 >= 0; i2--) heapBubble(this.heap, i2,);
    if ((this.to - pos || this.value.endSide - side) < 0) this.next();
  }
  next() {
    if (this.heap.length == 0) {
      this.from = this.to = 1e9;
      this.value = null;
      this.rank = -1;
    } else {
      let top3 = this.heap[0];
      this.from = top3.from;
      this.to = top3.to;
      this.value = top3.value;
      this.rank = top3.rank;
      if (top3.value) top3.next();
      heapBubble(this.heap, 0,);
    }
  }
  constructor(heap,) {
    this.heap = heap;
  }
};
function heapBubble(heap, index,) {
  for (let cur = heap[index];;) {
    let childIndex = (index << 1) + 1;
    if (childIndex >= heap.length) break;
    let child = heap[childIndex];
    if (childIndex + 1 < heap.length && child.compare(heap[childIndex + 1],) >= 0) {
      child = heap[childIndex + 1];
      childIndex++;
    }
    if (cur.compare(child,) < 0) break;
    heap[childIndex] = cur;
    heap[index] = child;
    index = childIndex;
  }
}
var SpanCursor = class {
  goto(pos, side = -1e9,) {
    this.cursor.goto(pos, side,);
    this.active.length = this.activeTo.length = this.activeRank.length = 0;
    this.minActive = -1;
    this.to = pos;
    this.endSide = side;
    this.openStart = -1;
    this.next();
    return this;
  }
  forward(pos, side,) {
    while (this.minActive > -1 && (this.activeTo[this.minActive] - pos || this.active[this.minActive].endSide - side) < 0) {
      this.removeActive(this.minActive,);
    }
    this.cursor.forward(pos, side,);
  }
  removeActive(index,) {
    remove(this.active, index,);
    remove(this.activeTo, index,);
    remove(this.activeRank, index,);
    this.minActive = findMinIndex(this.active, this.activeTo,);
  }
  addActive(trackOpen,) {
    let i2 = 0, { value, to, rank, } = this.cursor;
    while (i2 < this.activeRank.length && this.activeRank[i2] <= rank) i2++;
    insert(this.active, i2, value,);
    insert(this.activeTo, i2, to,);
    insert(this.activeRank, i2, rank,);
    if (trackOpen) insert(trackOpen, i2, this.cursor.from,);
    this.minActive = findMinIndex(this.active, this.activeTo,);
  }
  // After calling this, if `this.point` != null, the next range is a
  // point. Otherwise, it's a regular range, covered by `this.active`.
  next() {
    let from = this.to, wasPoint = this.point;
    this.point = null;
    let trackOpen = this.openStart < 0 ? [] : null;
    for (;;) {
      let a = this.minActive;
      if (a > -1 && (this.activeTo[a] - this.cursor.from || this.active[a].endSide - this.cursor.startSide) < 0) {
        if (this.activeTo[a] > from) {
          this.to = this.activeTo[a];
          this.endSide = this.active[a].endSide;
          break;
        }
        this.removeActive(a,);
        if (trackOpen) remove(trackOpen, a,);
      } else if (!this.cursor.value) {
        this.to = this.endSide = 1e9;
        break;
      } else if (this.cursor.from > from) {
        this.to = this.cursor.from;
        this.endSide = this.cursor.startSide;
        break;
      } else {
        let nextVal = this.cursor.value;
        if (!nextVal.point) {
          this.addActive(trackOpen,);
          this.cursor.next();
        } else if (wasPoint && this.cursor.to == this.to && this.cursor.from < this.cursor.to) {
          this.cursor.next();
        } else {
          this.point = nextVal;
          this.pointFrom = this.cursor.from;
          this.pointRank = this.cursor.rank;
          this.to = this.cursor.to;
          this.endSide = nextVal.endSide;
          this.cursor.next();
          this.forward(this.to, this.endSide,);
          break;
        }
      }
    }
    if (trackOpen) {
      this.openStart = 0;
      for (let i2 = trackOpen.length - 1; i2 >= 0 && trackOpen[i2] < from; i2--) this.openStart++;
    }
  }
  activeForPoint(to,) {
    if (!this.active.length) return this.active;
    let active = [];
    for (let i2 = this.active.length - 1; i2 >= 0; i2--) {
      if (this.activeRank[i2] < this.pointRank) break;
      if (this.activeTo[i2] > to || this.activeTo[i2] == to && this.active[i2].endSide >= this.point.endSide) active.push(this.active[i2],);
    }
    return active.reverse();
  }
  openEnd(to,) {
    let open = 0;
    for (let i2 = this.activeTo.length - 1; i2 >= 0 && this.activeTo[i2] > to; i2--) open++;
    return open;
  }
  constructor(sets, skip, minPoint,) {
    this.minPoint = minPoint;
    this.active = [];
    this.activeTo = [];
    this.activeRank = [];
    this.minActive = -1;
    this.point = null;
    this.pointFrom = 0;
    this.pointRank = 0;
    this.to = -1e9;
    this.endSide = 0;
    this.openStart = -1;
    this.cursor = HeapCursor.from(sets, skip, minPoint,);
  }
};
function compare(a, startA, b, startB, length, comparator,) {
  a.goto(startA,);
  b.goto(startB,);
  let endB = startB + length;
  let pos = startB, dPos = startB - startA;
  for (;;) {
    let diff = a.to + dPos - b.to || a.endSide - b.endSide;
    let end = diff < 0 ? a.to + dPos : b.to, clipEnd = Math.min(end, endB,);
    if (a.point || b.point) {
      if (
        !(a.point && b.point && (a.point == b.point || a.point.eq(b.point,)) &&
          sameValues(a.activeForPoint(a.to,), b.activeForPoint(b.to,),))
      ) comparator.comparePoint(pos, clipEnd, a.point, b.point,);
    } else {
      if (clipEnd > pos && !sameValues(a.active, b.active,)) comparator.compareRange(pos, clipEnd, a.active, b.active,);
    }
    if (end > endB) break;
    pos = end;
    if (diff <= 0) a.next();
    if (diff >= 0) b.next();
  }
}
function sameValues(a, b,) {
  if (a.length != b.length) return false;
  for (let i2 = 0; i2 < a.length; i2++) if (a[i2] != b[i2] && !a[i2].eq(b[i2],)) return false;
  return true;
}
function remove(array, index,) {
  for (let i2 = index, e = array.length - 1; i2 < e; i2++) array[i2] = array[i2 + 1];
  array.pop();
}
function insert(array, index, value,) {
  for (let i2 = array.length - 1; i2 >= index; i2--) array[i2 + 1] = array[i2];
  array[index] = value;
}
function findMinIndex(value, array,) {
  let found = -1, foundPos = 1e9;
  for (let i2 = 0; i2 < array.length; i2++) {
    if ((array[i2] - foundPos || value[i2].endSide - value[found].endSide) < 0) {
      found = i2;
      foundPos = array[i2];
    }
  }
  return found;
}
function countColumn(string2, tabSize, to = string2.length,) {
  let n = 0;
  for (let i2 = 0; i2 < to;) {
    if (string2.charCodeAt(i2,) == 9) {
      n += tabSize - n % tabSize;
      i2++;
    } else {
      n++;
      i2 = findClusterBreak(string2, i2,);
    }
  }
  return n;
}
function findColumn(string2, col, tabSize, strict,) {
  for (let i2 = 0, n = 0;;) {
    if (n >= col) return i2;
    if (i2 == string2.length) break;
    n += string2.charCodeAt(i2,) == 9 ? tabSize - n % tabSize : 1;
    i2 = findClusterBreak(string2, i2,);
  }
  return strict === true ? -1 : string2.length;
}

// https :https://framerusercontent.com/modules/wKCR2JyUW8wZdYweJrou/EndxaPZLF2o4GndkIvF9/codemirror_view.js
var C = '\u037C';
var COUNT = typeof Symbol == 'undefined' ? '__' + C : Symbol.for(C,);
var SET = typeof Symbol == 'undefined' ? '__styleSet' + Math.floor(Math.random() * 1e8,) : Symbol('styleSet',);
var top = typeof globalThis != 'undefined' ? globalThis : typeof window != 'undefined' ? window : {};
var StyleModule = class {
  // :: () → string
  // Returns a string containing the module's CSS rules.
  getRules() {
    return this.rules.join('\n',);
  }
  // :: () → string
  // Generate a new unique CSS class name.
  static newName() {
    let id2 = top[COUNT] || 1;
    top[COUNT] = id2 + 1;
    return C + id2.toString(36,);
  }
  // :: (union<Document, ShadowRoot>, union<[StyleModule], StyleModule>)
  //
  // Mount the given set of modules in the given DOM root, which ensures
  // that the CSS rules defined by the module are available in that
  // context.
  //
  // Rules are only added to the document once per root.
  //
  // Rule order will follow the order of the modules, so that rules from
  // modules later in the array take precedence of those from earlier
  // modules. If you call this function multiple times for the same root
  // in a way that changes the order of already mounted modules, the old
  // order will be changed.
  static mount(root, modules,) {
    (root[SET] || new StyleSet(root,)).mount(Array.isArray(modules,) ? modules : [modules,],);
  }
  // :: (Object<Style>, ?{finish: ?(string) → string})
  // Create a style module from the given spec.
  //
  // When `finish` is given, it is called on regular (non-`@`)
  // selectors (after `&` expansion) to compute the final selector.
  constructor(spec, options,) {
    this.rules = [];
    let { finish, } = options || {};
    function splitSelector(selector,) {
      return /^@/.test(selector,) ? [selector,] : selector.split(/,\s*/,);
    }
    function render(selectors, spec2, target, isKeyframes,) {
      let local = [], isAt = /^@(\w+)\b/.exec(selectors[0],), keyframes = isAt && isAt[1] == 'keyframes';
      if (isAt && spec2 == null) return target.push(selectors[0] + ';',);
      for (let prop in spec2) {
        let value = spec2[prop];
        if (/&/.test(prop,)) {
          render(
            prop.split(/,\s*/,).map((part,) => selectors.map((sel,) => part.replace(/&/, sel,))).reduce((a, b,) => a.concat(b,)),
            value,
            target,
          );
        } else if (value && typeof value == 'object') {
          if (!isAt) throw new RangeError('The value of a property (' + prop + ') should be a primitive value.',);
          render(splitSelector(prop,), value, local, keyframes,);
        } else if (value != null) {
          local.push(prop.replace(/_.*/, '',).replace(/[A-Z]/g, (l,) => '-' + l.toLowerCase(),) + ': ' + value + ';',);
        }
      }
      if (local.length || keyframes) {
        target.push((finish && !isAt && !isKeyframes ? selectors.map(finish,) : selectors).join(', ',) + ' {' + local.join(' ',) + '}',);
      }
    }
    for (let prop in spec) render(splitSelector(prop,), spec[prop], this.rules,);
  }
};
var adoptedSet = /* @__PURE__ */ new Map();
var StyleSet = class {
  mount(modules,) {
    let sheet = this.sheet;
    let pos = 0, j = 0;
    for (let i2 = 0; i2 < modules.length; i2++) {
      let mod = modules[i2], index = this.modules.indexOf(mod,);
      if (index < j && index > -1) {
        this.modules.splice(index, 1,);
        j--;
        index = -1;
      }
      if (index == -1) {
        this.modules.splice(j++, 0, mod,);
        if (sheet) for (let k = 0; k < mod.rules.length; k++) sheet.insertRule(mod.rules[k], pos++,);
      } else {
        while (j < index) pos += this.modules[j++].rules.length;
        pos += mod.rules.length;
        j++;
      }
    }
    if (!sheet) {
      let text = '';
      for (let i1 = 0; i1 < this.modules.length; i1++) text += this.modules[i1].getRules() + '\n';
      this.styleTag.textContent = text;
    }
  }
  constructor(root,) {
    let doc2 = root.ownerDocument || root, win = doc2.defaultView;
    if (!root.head && root.adoptedStyleSheets && win.CSSStyleSheet) {
      let adopted = adoptedSet.get(doc2,);
      if (adopted) {
        root.adoptedStyleSheets = [adopted.sheet, ...root.adoptedStyleSheets,];
        return root[SET] = adopted;
      }
      this.sheet = new win.CSSStyleSheet();
      root.adoptedStyleSheets = [this.sheet, ...root.adoptedStyleSheets,];
      adoptedSet.set(doc2, this,);
    } else {
      this.styleTag = doc2.createElement('style',);
      let target = root.head || root;
      target.insertBefore(this.styleTag, target.firstChild,);
    }
    this.modules = [];
    root[SET] = this;
  }
};
var base = {
  8: 'Backspace',
  9: 'Tab',
  10: 'Enter',
  12: 'NumLock',
  13: 'Enter',
  16: 'Shift',
  17: 'Control',
  18: 'Alt',
  20: 'CapsLock',
  27: 'Escape',
  32: ' ',
  33: 'PageUp',
  34: 'PageDown',
  35: 'End',
  36: 'Home',
  37: 'ArrowLeft',
  38: 'ArrowUp',
  39: 'ArrowRight',
  40: 'ArrowDown',
  44: 'PrintScreen',
  45: 'Insert',
  46: 'Delete',
  59: ';',
  61: '=',
  91: 'Meta',
  92: 'Meta',
  106: '*',
  107: '+',
  108: ',',
  109: '-',
  110: '.',
  111: '/',
  144: 'NumLock',
  145: 'ScrollLock',
  160: 'Shift',
  161: 'Shift',
  162: 'Control',
  163: 'Control',
  164: 'Alt',
  165: 'Alt',
  173: '-',
  186: ';',
  187: '=',
  188: ',',
  189: '-',
  190: '.',
  191: '/',
  192: '`',
  219: '[',
  220: '\\',
  221: ']',
  222: '\'',
};
var shift = {
  48: ')',
  49: '!',
  50: '@',
  51: '#',
  52: '$',
  53: '%',
  54: '^',
  55: '&',
  56: '*',
  57: '(',
  59: ':',
  61: '+',
  173: '_',
  186: ':',
  187: '+',
  188: '<',
  189: '_',
  190: '>',
  191: '?',
  192: '~',
  219: '{',
  220: '|',
  221: '}',
  222: '"',
};
var mac = typeof navigator != 'undefined' && /Mac/.test(navigator.platform,);
var ie = typeof navigator != 'undefined' && /MSIE \d|Trident\/(?:[7-9]|\d{2,})\..*rv:(\d+)/.exec(navigator.userAgent,);
for (i = 0; i < 10; i++) base[48 + i] = base[96 + i] = String(i,);
var i;
for (i = 1; i <= 24; i++) base[i + 111] = 'F' + i;
var i;
for (i = 65; i <= 90; i++) {
  base[i] = String.fromCharCode(i + 32,);
  shift[i] = String.fromCharCode(i,);
}
var i;
for (code in base) if (!shift.hasOwnProperty(code,)) shift[code] = base[code];
var code;
function keyName(event,) {
  var ignoreKey = mac && event.metaKey && event.shiftKey && !event.ctrlKey && !event.altKey ||
    ie && event.shiftKey && event.key && event.key.length == 1 || event.key == 'Unidentified';
  var name2 = !ignoreKey && event.key || (event.shiftKey ? shift : base)[event.keyCode] || event.key || 'Unidentified';
  if (name2 == 'Esc') name2 = 'Escape';
  if (name2 == 'Del') name2 = 'Delete';
  if (name2 == 'Left') name2 = 'ArrowLeft';
  if (name2 == 'Up') name2 = 'ArrowUp';
  if (name2 == 'Right') name2 = 'ArrowRight';
  if (name2 == 'Down') name2 = 'ArrowDown';
  return name2;
}
function getSelection(root,) {
  let target;
  if (root.nodeType == 11) {
    target = root.getSelection ? root : root.ownerDocument;
  } else {
    target = root;
  }
  return target.getSelection();
}
function contains(dom, node,) {
  return node ? dom == node || dom.contains(node.nodeType != 1 ? node.parentNode : node,) : false;
}
function deepActiveElement(doc2,) {
  let elt = doc2.activeElement;
  while (elt && elt.shadowRoot) elt = elt.shadowRoot.activeElement;
  return elt;
}
function hasSelection(dom, selection,) {
  if (!selection.anchorNode) return false;
  try {
    return contains(dom, selection.anchorNode,);
  } catch (_) {
    return false;
  }
}
function clientRectsFor(dom,) {
  if (dom.nodeType == 3) return textRange(dom, 0, dom.nodeValue.length,).getClientRects();
  else if (dom.nodeType == 1) return dom.getClientRects();
  else return [];
}
function isEquivalentPosition(node, off, targetNode, targetOff,) {
  return targetNode ? scanFor(node, off, targetNode, targetOff, -1,) || scanFor(node, off, targetNode, targetOff, 1,) : false;
}
function domIndex(node,) {
  for (var index = 0;; index++) {
    node = node.previousSibling;
    if (!node) return index;
  }
}
function scanFor(node, off, targetNode, targetOff, dir,) {
  for (;;) {
    if (node == targetNode && off == targetOff) return true;
    if (off == (dir < 0 ? 0 : maxOffset(node,))) {
      if (node.nodeName == 'DIV') return false;
      let parent = node.parentNode;
      if (!parent || parent.nodeType != 1) return false;
      off = domIndex(node,) + (dir < 0 ? 0 : 1);
      node = parent;
    } else if (node.nodeType == 1) {
      node = node.childNodes[off + (dir < 0 ? -1 : 0)];
      if (node.nodeType == 1 && node.contentEditable == 'false') return false;
      off = dir < 0 ? maxOffset(node,) : 0;
    } else {
      return false;
    }
  }
}
function maxOffset(node,) {
  return node.nodeType == 3 ? node.nodeValue.length : node.childNodes.length;
}
function flattenRect(rect, left,) {
  let x = left ? rect.left : rect.right;
  return { left: x, right: x, top: rect.top, bottom: rect.bottom, };
}
function windowRect(win,) {
  return { left: 0, right: win.innerWidth, top: 0, bottom: win.innerHeight, };
}
function scrollRectIntoView(dom, rect, side, x, y, xMargin, yMargin, ltr,) {
  let doc2 = dom.ownerDocument, win = doc2.defaultView || window;
  for (let cur = dom; cur;) {
    if (cur.nodeType == 1) {
      let bounding, top22 = cur == doc2.body;
      if (top22) {
        bounding = windowRect(win,);
      } else {
        if (cur.scrollHeight <= cur.clientHeight && cur.scrollWidth <= cur.clientWidth) {
          cur = cur.assignedSlot || cur.parentNode;
          continue;
        }
        let rect2 = cur.getBoundingClientRect();
        bounding = { left: rect2.left, right: rect2.left + cur.clientWidth, top: rect2.top, bottom: rect2.top + cur.clientHeight, };
      }
      let moveX = 0, moveY = 0;
      if (y == 'nearest') {
        if (rect.top < bounding.top) {
          moveY = -(bounding.top - rect.top + yMargin);
          if (side > 0 && rect.bottom > bounding.bottom + moveY) moveY = rect.bottom - bounding.bottom + moveY + yMargin;
        } else if (rect.bottom > bounding.bottom) {
          moveY = rect.bottom - bounding.bottom + yMargin;
          if (side < 0 && rect.top - moveY < bounding.top) moveY = -(bounding.top + moveY - rect.top + yMargin);
        }
      } else {
        let rectHeight = rect.bottom - rect.top, boundingHeight = bounding.bottom - bounding.top;
        let targetTop = y == 'center' && rectHeight <= boundingHeight
          ? rect.top + rectHeight / 2 - boundingHeight / 2
          : y == 'start' || y == 'center' && side < 0
          ? rect.top - yMargin
          : rect.bottom - boundingHeight + yMargin;
        moveY = targetTop - bounding.top;
      }
      if (x == 'nearest') {
        if (rect.left < bounding.left) {
          moveX = -(bounding.left - rect.left + xMargin);
          if (side > 0 && rect.right > bounding.right + moveX) moveX = rect.right - bounding.right + moveX + xMargin;
        } else if (rect.right > bounding.right) {
          moveX = rect.right - bounding.right + xMargin;
          if (side < 0 && rect.left < bounding.left + moveX) moveX = -(bounding.left + moveX - rect.left + xMargin);
        }
      } else {
        let targetLeft = x == 'center'
          ? rect.left + (rect.right - rect.left) / 2 - (bounding.right - bounding.left) / 2
          : x == 'start' == ltr
          ? rect.left - xMargin
          : rect.right - (bounding.right - bounding.left) + xMargin;
        moveX = targetLeft - bounding.left;
      }
      if (moveX || moveY) {
        if (top22) {
          win.scrollBy(moveX, moveY,);
        } else {
          let movedX = 0, movedY = 0;
          if (moveY) {
            let start = cur.scrollTop;
            cur.scrollTop += moveY;
            movedY = cur.scrollTop - start;
          }
          if (moveX) {
            let start1 = cur.scrollLeft;
            cur.scrollLeft += moveX;
            movedX = cur.scrollLeft - start1;
          }
          rect = { left: rect.left - movedX, top: rect.top - movedY, right: rect.right - movedX, bottom: rect.bottom - movedY, };
          if (movedX && Math.abs(movedX - moveX,) < 1) x = 'nearest';
          if (movedY && Math.abs(movedY - moveY,) < 1) y = 'nearest';
        }
      }
      if (top22) break;
      cur = cur.assignedSlot || cur.parentNode;
    } else if (cur.nodeType == 11) {
      cur = cur.host;
    } else {
      break;
    }
  }
}
function scrollableParent(dom,) {
  let doc2 = dom.ownerDocument;
  for (let cur = dom.parentNode; cur;) {
    if (cur == doc2.body) {
      break;
    } else if (cur.nodeType == 1) {
      if (cur.scrollHeight > cur.clientHeight || cur.scrollWidth > cur.clientWidth) return cur;
      cur = cur.assignedSlot || cur.parentNode;
    } else if (cur.nodeType == 11) {
      cur = cur.host;
    } else {
      break;
    }
  }
  return null;
}
var DOMSelectionState = class {
  eq(domSel,) {
    return this.anchorNode == domSel.anchorNode && this.anchorOffset == domSel.anchorOffset && this.focusNode == domSel.focusNode &&
      this.focusOffset == domSel.focusOffset;
  }
  setRange(range,) {
    let { anchorNode, focusNode, } = range;
    this.set(
      anchorNode,
      Math.min(range.anchorOffset, anchorNode ? maxOffset(anchorNode,) : 0,),
      focusNode,
      Math.min(range.focusOffset, focusNode ? maxOffset(focusNode,) : 0,),
    );
  }
  set(anchorNode, anchorOffset, focusNode, focusOffset,) {
    this.anchorNode = anchorNode;
    this.anchorOffset = anchorOffset;
    this.focusNode = focusNode;
    this.focusOffset = focusOffset;
  }
  constructor() {
    this.anchorNode = null;
    this.anchorOffset = 0;
    this.focusNode = null;
    this.focusOffset = 0;
  }
};
var preventScrollSupported = null;
function focusPreventScroll(dom,) {
  if (dom.setActive) return dom.setActive();
  if (preventScrollSupported) return dom.focus(preventScrollSupported,);
  let stack = [];
  for (let cur = dom; cur; cur = cur.parentNode) {
    stack.push(cur, cur.scrollTop, cur.scrollLeft,);
    if (cur == cur.ownerDocument) break;
  }
  dom.focus(
    preventScrollSupported == null
      ? {
        get preventScroll() {
          preventScrollSupported = { preventScroll: true, };
          return true;
        },
      }
      : void 0,
  );
  if (!preventScrollSupported) {
    preventScrollSupported = false;
    for (let i2 = 0; i2 < stack.length;) {
      let elt = stack[i2++], top22 = stack[i2++], left = stack[i2++];
      if (elt.scrollTop != top22) elt.scrollTop = top22;
      if (elt.scrollLeft != left) elt.scrollLeft = left;
    }
  }
}
var scratchRange;
function textRange(node, from, to = from,) {
  let range = scratchRange || (scratchRange = document.createRange());
  range.setEnd(node, to,);
  range.setStart(node, from,);
  return range;
}
function dispatchKey(elt, name2, code2,) {
  let options = { key: name2, code: name2, keyCode: code2, which: code2, cancelable: true, };
  let down = new KeyboardEvent('keydown', options,);
  down.synthetic = true;
  elt.dispatchEvent(down,);
  let up = new KeyboardEvent('keyup', options,);
  up.synthetic = true;
  elt.dispatchEvent(up,);
  return down.defaultPrevented || up.defaultPrevented;
}
function getRoot(node,) {
  while (node) {
    if (node && (node.nodeType == 9 || node.nodeType == 11 && node.host)) return node;
    node = node.assignedSlot || node.parentNode;
  }
  return null;
}
function clearAttributes(node,) {
  while (node.attributes.length) node.removeAttributeNode(node.attributes[0],);
}
function atElementStart(doc2, selection,) {
  let node = selection.focusNode, offset = selection.focusOffset;
  if (!node || selection.anchorNode != node || selection.anchorOffset != offset) return false;
  offset = Math.min(offset, maxOffset(node,),);
  for (;;) {
    if (offset) {
      if (node.nodeType != 1) return false;
      let prev = node.childNodes[offset - 1];
      if (prev.contentEditable == 'false') offset--;
      else {
        node = prev;
        offset = maxOffset(node,);
      }
    } else if (node == doc2) {
      return true;
    } else {
      offset = domIndex(node,);
      node = node.parentNode;
    }
  }
}
var DOMPos = class {
  static before(dom, precise,) {
    return new DOMPos(dom.parentNode, domIndex(dom,), precise,);
  }
  static after(dom, precise,) {
    return new DOMPos(dom.parentNode, domIndex(dom,) + 1, precise,);
  }
  constructor(node, offset, precise = true,) {
    this.node = node;
    this.offset = offset;
    this.precise = precise;
  }
};
var noChildren = [];
var ContentView = class {
  get overrideDOMText() {
    return null;
  }
  get posAtStart() {
    return this.parent ? this.parent.posBefore(this,) : 0;
  }
  get posAtEnd() {
    return this.posAtStart + this.length;
  }
  posBefore(view,) {
    let pos = this.posAtStart;
    for (let child of this.children) {
      if (child == view) return pos;
      pos += child.length + child.breakAfter;
    }
    throw new RangeError('Invalid child in posBefore',);
  }
  posAfter(view,) {
    return this.posBefore(view,) + view.length;
  }
  sync(view, track,) {
    if (this.dirty & 2) {
      let parent = this.dom;
      let prev = null, next;
      for (let child of this.children) {
        if (child.dirty) {
          if (!child.dom && (next = prev ? prev.nextSibling : parent.firstChild)) {
            let contentView = ContentView.get(next,);
            if (!contentView || !contentView.parent && contentView.canReuseDOM(child,)) child.reuseDOM(next,);
          }
          child.sync(view, track,);
          child.dirty = 0;
        }
        next = prev ? prev.nextSibling : parent.firstChild;
        if (track && !track.written && track.node == parent && next != child.dom) track.written = true;
        if (child.dom.parentNode == parent) {
          while (next && next != child.dom) next = rm$1(next,);
        } else {
          parent.insertBefore(child.dom, next,);
        }
        prev = child.dom;
      }
      next = prev ? prev.nextSibling : parent.firstChild;
      if (next && track && track.node == parent) track.written = true;
      while (next) next = rm$1(next,);
    } else if (this.dirty & 1) {
      for (let child1 of this.children) {
        if (child1.dirty) {
          child1.sync(view, track,);
          child1.dirty = 0;
        }
      }
    }
  }
  reuseDOM(_dom,) {
  }
  localPosFromDOM(node, offset,) {
    let after;
    if (node == this.dom) {
      after = this.dom.childNodes[offset];
    } else {
      let bias = maxOffset(node,) == 0 ? 0 : offset == 0 ? -1 : 1;
      for (;;) {
        let parent = node.parentNode;
        if (parent == this.dom) break;
        if (bias == 0 && parent.firstChild != parent.lastChild) {
          if (node == parent.firstChild) bias = -1;
          else bias = 1;
        }
        node = parent;
      }
      if (bias < 0) after = node;
      else after = node.nextSibling;
    }
    if (after == this.dom.firstChild) return 0;
    while (after && !ContentView.get(after,)) after = after.nextSibling;
    if (!after) return this.length;
    for (let i2 = 0, pos = 0;; i2++) {
      let child = this.children[i2];
      if (child.dom == after) return pos;
      pos += child.length + child.breakAfter;
    }
  }
  domBoundsAround(from, to, offset = 0,) {
    let fromI = -1, fromStart = -1, toI = -1, toEnd = -1;
    for (let i2 = 0, pos = offset, prevEnd = offset; i2 < this.children.length; i2++) {
      let child = this.children[i2], end = pos + child.length;
      if (pos < from && end > to) return child.domBoundsAround(from, to, pos,);
      if (end >= from && fromI == -1) {
        fromI = i2;
        fromStart = pos;
      }
      if (pos > to && child.dom.parentNode == this.dom) {
        toI = i2;
        toEnd = prevEnd;
        break;
      }
      prevEnd = end;
      pos = end + child.breakAfter;
    }
    return {
      from: fromStart,
      to: toEnd < 0 ? offset + this.length : toEnd,
      startDOM: (fromI ? this.children[fromI - 1].dom.nextSibling : null) || this.dom.firstChild,
      endDOM: toI < this.children.length && toI >= 0 ? this.children[toI].dom : null,
    };
  }
  markDirty(andParent = false,) {
    this.dirty |= 2;
    this.markParentsDirty(andParent,);
  }
  markParentsDirty(childList,) {
    for (let parent = this.parent; parent; parent = parent.parent) {
      if (childList) parent.dirty |= 2;
      if (parent.dirty & 1) return;
      parent.dirty |= 1;
      childList = false;
    }
  }
  setParent(parent,) {
    if (this.parent != parent) {
      this.parent = parent;
      if (this.dirty) this.markParentsDirty(true,);
    }
  }
  setDOM(dom,) {
    if (this.dom) this.dom.cmView = null;
    this.dom = dom;
    dom.cmView = this;
  }
  get rootView() {
    for (let v = this;;) {
      let parent = v.parent;
      if (!parent) return v;
      v = parent;
    }
  }
  replaceChildren(from, to, children = noChildren,) {
    this.markDirty();
    for (let i2 = from; i2 < to; i2++) {
      let child = this.children[i2];
      if (child.parent == this) child.destroy();
    }
    this.children.splice(from, to - from, ...children,);
    for (let i1 = 0; i1 < children.length; i1++) children[i1].setParent(this,);
  }
  ignoreMutation(_rec,) {
    return false;
  }
  ignoreEvent(_event,) {
    return false;
  }
  childCursor(pos = this.length,) {
    return new ChildCursor(this.children, pos, this.children.length,);
  }
  childPos(pos, bias = 1,) {
    return this.childCursor().findPos(pos, bias,);
  }
  toString() {
    let name2 = this.constructor.name.replace('View', '',);
    return name2 + (this.children.length
      ? '(' + this.children.join() + ')'
      : this.length
      ? '[' + (name2 == 'Text' ? this.text : this.length) + ']'
      : '') +
      (this.breakAfter ? '#' : '');
  }
  static get(node,) {
    return node.cmView;
  }
  get isEditable() {
    return true;
  }
  get isWidget() {
    return false;
  }
  get isHidden() {
    return false;
  }
  merge(from, to, source, hasStart, openStart, openEnd,) {
    return false;
  }
  become(other,) {
    return false;
  }
  canReuseDOM(other,) {
    return other.constructor == this.constructor;
  }
  // When this is a zero-length view with a side, this should return a
  // number <= 0 to indicate it is before its position, or a
  // number > 0 when after its position.
  getSide() {
    return 0;
  }
  destroy() {
    this.parent = null;
  }
  constructor() {
    this.parent = null;
    this.dom = null;
    this.dirty = 2;
  }
};
ContentView.prototype.breakAfter = 0;
function rm$1(dom,) {
  let next = dom.nextSibling;
  dom.parentNode.removeChild(dom,);
  return next;
}
var ChildCursor = class {
  findPos(pos, bias = 1,) {
    for (;;) {
      if (pos > this.pos || pos == this.pos && (bias > 0 || this.i == 0 || this.children[this.i - 1].breakAfter)) {
        this.off = pos - this.pos;
        return this;
      }
      let next = this.children[--this.i];
      this.pos -= next.length + next.breakAfter;
    }
  }
  constructor(children, pos, i2,) {
    this.children = children;
    this.pos = pos;
    this.i = i2;
    this.off = 0;
  }
};
function replaceRange(parent, fromI, fromOff, toI, toOff, insert2, breakAtStart, openStart, openEnd,) {
  let { children, } = parent;
  let before = children.length ? children[fromI] : null;
  let last = insert2.length ? insert2[insert2.length - 1] : null;
  let breakAtEnd = last ? last.breakAfter : breakAtStart;
  if (
    fromI == toI && before && !breakAtStart && !breakAtEnd && insert2.length < 2 &&
    before.merge(fromOff, toOff, insert2.length ? last : null, fromOff == 0, openStart, openEnd,)
  ) return;
  if (toI < children.length) {
    let after = children[toI];
    if (after && toOff < after.length) {
      if (fromI == toI) {
        after = after.split(toOff,);
        toOff = 0;
      }
      if (!breakAtEnd && last && after.merge(0, toOff, last, true, 0, openEnd,)) {
        insert2[insert2.length - 1] = after;
      } else {
        if (toOff) after.merge(0, toOff, null, false, 0, openEnd,);
        insert2.push(after,);
      }
    } else if (after === null || after === void 0 ? void 0 : after.breakAfter) {
      if (last) last.breakAfter = 1;
      else breakAtStart = 1;
    }
    toI++;
  }
  if (before) {
    before.breakAfter = breakAtStart;
    if (fromOff > 0) {
      if (!breakAtStart && insert2.length && before.merge(fromOff, before.length, insert2[0], false, openStart, 0,)) {
        before.breakAfter = insert2.shift().breakAfter;
      } else if (fromOff < before.length || before.children.length && before.children[before.children.length - 1].length == 0) {
        before.merge(fromOff, before.length, null, false, openStart, 0,);
      }
      fromI++;
    }
  }
  while (fromI < toI && insert2.length) {
    if (children[toI - 1].become(insert2[insert2.length - 1],)) {
      toI--;
      insert2.pop();
      openEnd = insert2.length ? 0 : openStart;
    } else if (children[fromI].become(insert2[0],)) {
      fromI++;
      insert2.shift();
      openStart = insert2.length ? 0 : openEnd;
    } else {
      break;
    }
  }
  if (
    !insert2.length && fromI && toI < children.length && !children[fromI - 1].breakAfter &&
    children[toI].merge(0, 0, children[fromI - 1], false, openStart, openEnd,)
  ) fromI--;
  if (fromI < toI || insert2.length) parent.replaceChildren(fromI, toI, insert2,);
}
function mergeChildrenInto(parent, from, to, insert2, openStart, openEnd,) {
  let cur = parent.childCursor();
  let { i: toI, off: toOff, } = cur.findPos(to, 1,);
  let { i: fromI, off: fromOff, } = cur.findPos(from, -1,);
  let dLen = from - to;
  for (let view of insert2) dLen += view.length;
  parent.length += dLen;
  replaceRange(parent, fromI, fromOff, toI, toOff, insert2, 0, openStart, openEnd,);
}
var nav = typeof navigator != 'undefined' ? navigator : { userAgent: '', vendor: '', platform: '', };
var doc = typeof document != 'undefined' ? document : { documentElement: { style: {}, }, };
var ie_edge = /* @__PURE__ */ /Edge\/(\d+)/.exec(nav.userAgent,);
var ie_upto10 = /* @__PURE__ */ /MSIE \d/.test(nav.userAgent,);
var ie_11up = /* @__PURE__ */ /Trident\/(?:[7-9]|\d{2,})\..*rv:(\d+)/.exec(nav.userAgent,);
var ie2 = !!(ie_upto10 || ie_11up || ie_edge);
var gecko = !ie2 && /* @__PURE__ */ /gecko\/(\d+)/i.test(nav.userAgent,);
var chrome = !ie2 && /* @__PURE__ */ /Chrome\/(\d+)/.exec(nav.userAgent,);
var webkit = 'webkitFontSmoothing' in doc.documentElement.style;
var safari = !ie2 && /* @__PURE__ */ /Apple Computer/.test(nav.vendor,);
var ios = safari && (/Mobile\/\w+/.test(nav.userAgent,) || nav.maxTouchPoints > 2);
var browser = {
  mac: ios || /* @__PURE__ */ /Mac/.test(nav.platform,),
  windows: /* @__PURE__ */ /Win/.test(nav.platform,),
  linux: /* @__PURE__ */ /Linux|X11/.test(nav.platform,),
  ie: ie2,
  ie_version: ie_upto10 ? doc.documentMode || 6 : ie_11up ? +ie_11up[1] : ie_edge ? +ie_edge[1] : 0,
  gecko,
  gecko_version: gecko ? +(/Firefox\/(\d+)/.exec(nav.userAgent,) || [0, 0,])[1] : 0,
  chrome: !!chrome,
  chrome_version: chrome ? +chrome[1] : 0,
  ios,
  android: /* @__PURE__ */ /Android\b/.test(nav.userAgent,),
  webkit,
  safari,
  webkit_version: webkit ? +(/\bAppleWebKit\/(\d+)/.exec(navigator.userAgent,) || [0, 0,])[1] : 0,
  tabSize: doc.documentElement.style.tabSize != null ? 'tab-size' : '-moz-tab-size',
};
var MaxJoinLen = 256;
var TextView = class extends ContentView {
  get length() {
    return this.text.length;
  }
  createDOM(textDOM,) {
    this.setDOM(textDOM || document.createTextNode(this.text,),);
  }
  sync(view, track,) {
    if (!this.dom) this.createDOM();
    if (this.dom.nodeValue != this.text) {
      if (track && track.node == this.dom) track.written = true;
      this.dom.nodeValue = this.text;
    }
  }
  reuseDOM(dom,) {
    if (dom.nodeType == 3) this.createDOM(dom,);
  }
  merge(from, to, source,) {
    if (source && (!(source instanceof TextView) || this.length - (to - from) + source.length > MaxJoinLen)) return false;
    this.text = this.text.slice(0, from,) + (source ? source.text : '') + this.text.slice(to,);
    this.markDirty();
    return true;
  }
  split(from,) {
    let result = new TextView(this.text.slice(from,),);
    this.text = this.text.slice(0, from,);
    this.markDirty();
    return result;
  }
  localPosFromDOM(node, offset,) {
    return node == this.dom ? offset : offset ? this.text.length : 0;
  }
  domAtPos(pos,) {
    return new DOMPos(this.dom, pos,);
  }
  domBoundsAround(_from, _to, offset,) {
    return { from: offset, to: offset + this.length, startDOM: this.dom, endDOM: this.dom.nextSibling, };
  }
  coordsAt(pos, side,) {
    return textCoords(this.dom, pos, side,);
  }
  constructor(text,) {
    super();
    this.text = text;
  }
};
var MarkView = class extends ContentView {
  setAttrs(dom,) {
    clearAttributes(dom,);
    if (this.mark.class) dom.className = this.mark.class;
    if (this.mark.attrs) for (let name2 in this.mark.attrs) dom.setAttribute(name2, this.mark.attrs[name2],);
    return dom;
  }
  reuseDOM(node,) {
    if (node.nodeName == this.mark.tagName.toUpperCase()) {
      this.setDOM(node,);
      this.dirty |= 4 | 2;
    }
  }
  sync(view, track,) {
    if (!this.dom) this.setDOM(this.setAttrs(document.createElement(this.mark.tagName,),),);
    else if (this.dirty & 4) this.setAttrs(this.dom,);
    super.sync(view, track,);
  }
  merge(from, to, source, _hasStart, openStart, openEnd,) {
    if (
      source && (!(source instanceof MarkView && source.mark.eq(this.mark,)) || from && openStart <= 0 || to < this.length && openEnd <= 0)
    ) return false;
    mergeChildrenInto(this, from, to, source ? source.children : [], openStart - 1, openEnd - 1,);
    this.markDirty();
    return true;
  }
  split(from,) {
    let result = [], off = 0, detachFrom = -1, i2 = 0;
    for (let elt of this.children) {
      let end = off + elt.length;
      if (end > from) result.push(off < from ? elt.split(from - off,) : elt,);
      if (detachFrom < 0 && off >= from) detachFrom = i2;
      off = end;
      i2++;
    }
    let length = this.length - from;
    this.length = from;
    if (detachFrom > -1) {
      this.children.length = detachFrom;
      this.markDirty();
    }
    return new MarkView(this.mark, result, length,);
  }
  domAtPos(pos,) {
    return inlineDOMAtPos(this, pos,);
  }
  coordsAt(pos, side,) {
    return coordsInChildren(this, pos, side,);
  }
  constructor(mark, children = [], length = 0,) {
    super();
    this.mark = mark;
    this.children = children;
    this.length = length;
    for (let ch of children) ch.setParent(this,);
  }
};
function textCoords(text, pos, side,) {
  let length = text.nodeValue.length;
  if (pos > length) pos = length;
  let from = pos, to = pos, flatten2 = 0;
  if (pos == 0 && side < 0 || pos == length && side >= 0) {
    if (!(browser.chrome || browser.gecko)) {
      if (pos) {
        from--;
        flatten2 = 1;
      } else if (to < length) {
        to++;
        flatten2 = -1;
      }
    }
  } else {
    if (side < 0) from--;
    else if (to < length) to++;
  }
  let rects = textRange(text, from, to,).getClientRects();
  if (!rects.length) return null;
  let rect = rects[(flatten2 ? flatten2 < 0 : side >= 0) ? 0 : rects.length - 1];
  if (browser.safari && !flatten2 && rect.width == 0) rect = Array.prototype.find.call(rects, (r,) => r.width,) || rect;
  return flatten2 ? flattenRect(rect, flatten2 < 0,) : rect || null;
}
var WidgetView = class extends ContentView {
  static create(widget, length, side,) {
    return new (widget.customView || WidgetView)(widget, length, side,);
  }
  split(from,) {
    let result = WidgetView.create(this.widget, this.length - from, this.side,);
    this.length -= from;
    return result;
  }
  sync(view,) {
    if (!this.dom || !this.widget.updateDOM(this.dom, view,)) {
      if (this.dom && this.prevWidget) this.prevWidget.destroy(this.dom,);
      this.prevWidget = null;
      this.setDOM(this.widget.toDOM(view,),);
      this.dom.contentEditable = 'false';
    }
  }
  getSide() {
    return this.side;
  }
  merge(from, to, source, hasStart, openStart, openEnd,) {
    if (
      source &&
      (!(source instanceof WidgetView) || !this.widget.compare(source.widget,) || from > 0 && openStart <= 0 ||
        to < this.length && openEnd <= 0)
    ) return false;
    this.length = from + (source ? source.length : 0) + (this.length - to);
    return true;
  }
  become(other,) {
    if (other instanceof WidgetView && other.side == this.side && this.widget.constructor == other.widget.constructor) {
      if (!this.widget.compare(other.widget,)) this.markDirty(true,);
      if (this.dom && !this.prevWidget) this.prevWidget = this.widget;
      this.widget = other.widget;
      this.length = other.length;
      return true;
    }
    return false;
  }
  ignoreMutation() {
    return true;
  }
  ignoreEvent(event,) {
    return this.widget.ignoreEvent(event,);
  }
  get overrideDOMText() {
    if (this.length == 0) return Text.empty;
    let top22 = this;
    while (top22.parent) top22 = top22.parent;
    let { view, } = top22, text = view && view.state.doc, start = this.posAtStart;
    return text ? text.slice(start, start + this.length,) : Text.empty;
  }
  domAtPos(pos,) {
    return (this.length ? pos == 0 : this.side > 0) ? DOMPos.before(this.dom,) : DOMPos.after(this.dom, pos == this.length,);
  }
  domBoundsAround() {
    return null;
  }
  coordsAt(pos, side,) {
    let custom = this.widget.coordsAt(this.dom, pos, side,);
    if (custom) return custom;
    let rects = this.dom.getClientRects(), rect = null;
    if (!rects.length) return null;
    let fromBack = this.side ? this.side < 0 : pos > 0;
    for (let i2 = fromBack ? rects.length - 1 : 0;; i2 += fromBack ? -1 : 1) {
      rect = rects[i2];
      if (pos > 0 ? i2 == 0 : i2 == rects.length - 1 || rect.top < rect.bottom) break;
    }
    return this.length ? rect : flattenRect(rect, !fromBack,);
  }
  get isEditable() {
    return false;
  }
  get isWidget() {
    return true;
  }
  get isHidden() {
    return this.widget.isHidden;
  }
  destroy() {
    super.destroy();
    if (this.dom) this.widget.destroy(this.dom,);
  }
  constructor(widget, length, side,) {
    super();
    this.widget = widget;
    this.length = length;
    this.side = side;
    this.prevWidget = null;
  }
};
var CompositionView = class extends WidgetView {
  domAtPos(pos,) {
    let { topView, text, } = this.widget;
    if (!topView) return new DOMPos(text, Math.min(pos, text.nodeValue.length,),);
    return scanCompositionTree(
      pos,
      0,
      topView,
      text,
      this.length - topView.length,
      (v, p,) => v.domAtPos(p,),
      (text2, p,) => new DOMPos(text2, Math.min(p, text2.nodeValue.length,),),
    );
  }
  sync() {
    this.setDOM(this.widget.toDOM(),);
  }
  localPosFromDOM(node, offset,) {
    let { topView, text, } = this.widget;
    if (!topView) return Math.min(offset, this.length,);
    return posFromDOMInCompositionTree(node, offset, topView, text, this.length - topView.length,);
  }
  ignoreMutation() {
    return false;
  }
  get overrideDOMText() {
    return null;
  }
  coordsAt(pos, side,) {
    let { topView, text, } = this.widget;
    if (!topView) return textCoords(text, pos, side,);
    return scanCompositionTree(
      pos,
      side,
      topView,
      text,
      this.length - topView.length,
      (v, pos2, side2,) => v.coordsAt(pos2, side2,),
      (text2, pos2, side2,) => textCoords(text2, pos2, side2,),
    );
  }
  destroy() {
    var _a2;
    super.destroy();
    (_a2 = this.widget.topView) === null || _a2 === void 0 ? void 0 : _a2.destroy();
  }
  get isEditable() {
    return true;
  }
  canReuseDOM() {
    return true;
  }
};
function scanCompositionTree(pos, side, view, text, dLen, enterView, fromText,) {
  if (view instanceof MarkView) {
    for (let child = view.dom.firstChild; child; child = child.nextSibling) {
      let desc = ContentView.get(child,);
      if (!desc) {
        let inner = scanCompositionNode(pos, side, child, fromText,);
        if (typeof inner != 'number') return inner;
        pos = inner;
      } else {
        let hasComp = contains(child, text,);
        let len = desc.length + (hasComp ? dLen : 0);
        if (pos < len || pos == len && desc.getSide() <= 0) {
          return hasComp
            ? scanCompositionTree(pos, side, desc, text, dLen, enterView, fromText,)
            : enterView(desc, pos, side,);
        }
        pos -= len;
      }
    }
    return enterView(view, view.length, -1,);
  } else if (view.dom == text) {
    return fromText(text, pos, side,);
  } else {
    return enterView(view, pos, side,);
  }
}
function scanCompositionNode(pos, side, node, fromText,) {
  if (node.nodeType == 3) {
    let len = node.nodeValue.length;
    if (pos <= len) return fromText(node, pos, side,);
    pos -= len;
  } else if (node.nodeType == 1 && node.contentEditable != 'false') {
    for (let child = node.firstChild; child; child = child.nextSibling) {
      let inner = scanCompositionNode(pos, side, child, fromText,);
      if (typeof inner != 'number') return inner;
      pos = inner;
    }
  }
  return pos;
}
function posFromDOMInCompositionTree(node, offset, view, text, dLen,) {
  if (view instanceof MarkView) {
    let pos = 0;
    for (let child = view.dom.firstChild; child; child = child.nextSibling) {
      let childView = ContentView.get(child,);
      if (childView) {
        let hasComp = contains(child, text,);
        if (contains(child, node,)) {
          return pos +
            (hasComp ? posFromDOMInCompositionTree(node, offset, childView, text, dLen,) : childView.localPosFromDOM(node, offset,));
        }
        pos += childView.length + (hasComp ? dLen : 0);
      } else {
        let inner = posFromDOMInOpaqueNode(node, offset, child,);
        if (inner.result != null) return pos + inner.result;
        pos += inner.size;
      }
    }
  } else if (view.dom == text) {
    return Math.min(offset, text.nodeValue.length,);
  }
  return view.localPosFromDOM(node, offset,);
}
function posFromDOMInOpaqueNode(node, offset, target,) {
  if (target.nodeType == 3) {
    return node == target ? { result: offset, } : { size: target.nodeValue.length, };
  } else if (target.nodeType == 1 && target.contentEditable != 'false') {
    let pos = 0;
    for (let child = target.firstChild, i2 = 0;; child = child.nextSibling, i2++) {
      if (node == target && i2 == offset) return { result: pos, };
      if (!child) return { size: pos, };
      let inner = posFromDOMInOpaqueNode(node, offset, child,);
      if (inner.result != null) return { result: offset + inner.result, };
      pos += inner.size;
    }
  } else {
    return target.contains(node,) ? { result: 0, } : { size: 0, };
  }
}
var WidgetBufferView = class extends ContentView {
  get length() {
    return 0;
  }
  merge() {
    return false;
  }
  become(other,) {
    return other instanceof WidgetBufferView && other.side == this.side;
  }
  split() {
    return new WidgetBufferView(this.side,);
  }
  sync() {
    if (!this.dom) {
      let dom = document.createElement('img',);
      dom.className = 'cm-widgetBuffer';
      dom.setAttribute('aria-hidden', 'true',);
      this.setDOM(dom,);
    }
  }
  getSide() {
    return this.side;
  }
  domAtPos(pos,) {
    return this.side > 0 ? DOMPos.before(this.dom,) : DOMPos.after(this.dom,);
  }
  localPosFromDOM() {
    return 0;
  }
  domBoundsAround() {
    return null;
  }
  coordsAt(pos,) {
    return this.dom.getBoundingClientRect();
  }
  get overrideDOMText() {
    return Text.empty;
  }
  get isHidden() {
    return true;
  }
  constructor(side,) {
    super();
    this.side = side;
  }
};
TextView.prototype.children = WidgetView.prototype.children = WidgetBufferView.prototype.children = noChildren;
function inlineDOMAtPos(parent, pos,) {
  let dom = parent.dom, { children, } = parent, i2 = 0;
  for (let off = 0; i2 < children.length; i2++) {
    let child = children[i2], end = off + child.length;
    if (end == off && child.getSide() <= 0) continue;
    if (pos > off && pos < end && child.dom.parentNode == dom) return child.domAtPos(pos - off,);
    if (pos <= off) break;
    off = end;
  }
  for (let j = i2; j > 0; j--) {
    let prev = children[j - 1];
    if (prev.dom.parentNode == dom) return prev.domAtPos(prev.length,);
  }
  for (let j1 = i2; j1 < children.length; j1++) {
    let next = children[j1];
    if (next.dom.parentNode == dom) return next.domAtPos(0,);
  }
  return new DOMPos(dom, 0,);
}
function joinInlineInto(parent, view, open,) {
  let last, { children, } = parent;
  if (
    open > 0 && view instanceof MarkView && children.length && (last = children[children.length - 1]) instanceof MarkView &&
    last.mark.eq(view.mark,)
  ) {
    joinInlineInto(last, view.children[0], open - 1,);
  } else {
    children.push(view,);
    view.setParent(parent,);
  }
  parent.length += view.length;
}
function coordsInChildren(view, pos, side,) {
  let before = null, beforePos = -1, after = null, afterPos = -1;
  function scan(view2, pos2,) {
    for (let i2 = 0, off = 0; i2 < view2.children.length && off <= pos2; i2++) {
      let child = view2.children[i2], end = off + child.length;
      if (end >= pos2) {
        if (child.children.length) {
          scan(child, pos2 - off,);
        } else if ((!after || after.isHidden && side > 0) && (end > pos2 || off == end && child.getSide() > 0)) {
          after = child;
          afterPos = pos2 - off;
        } else if (off < pos2 || off == end && child.getSide() < 0 && !child.isHidden) {
          before = child;
          beforePos = pos2 - off;
        }
      }
      off = end;
    }
  }
  scan(view, pos,);
  let target = (side < 0 ? before : after) || before || after;
  if (target) return target.coordsAt(Math.max(0, target == before ? beforePos : afterPos,), side,);
  return fallbackRect(view,);
}
function fallbackRect(view,) {
  let last = view.dom.lastChild;
  if (!last) return view.dom.getBoundingClientRect();
  let rects = clientRectsFor(last,);
  return rects[rects.length - 1] || null;
}
function combineAttrs(source, target,) {
  for (let name2 in source) {
    if (name2 == 'class' && target.class) target.class += ' ' + source.class;
    else if (name2 == 'style' && target.style) target.style += ';' + source.style;
    else target[name2] = source[name2];
  }
  return target;
}
function attrsEq(a, b,) {
  if (a == b) return true;
  if (!a || !b) return false;
  let keysA = Object.keys(a,), keysB = Object.keys(b,);
  if (keysA.length != keysB.length) return false;
  for (let key of keysA) {
    if (keysB.indexOf(key,) == -1 || a[key] !== b[key]) return false;
  }
  return true;
}
function updateAttrs(dom, prev, attrs,) {
  let changed = null;
  if (prev) {
    for (let name2 in prev) if (!(attrs && name2 in attrs)) dom.removeAttribute(changed = name2,);
  }
  if (attrs) {
    for (let name1 in attrs) if (!(prev && prev[name1] == attrs[name1])) dom.setAttribute(changed = name1, attrs[name1],);
  }
  return !!changed;
}
var WidgetType = class {
  /**
  Compare this instance to another instance of the same type.
  (TypeScript can't express this, but only instances of the same
  specific class will be passed to this method.) This is used to
  avoid redrawing widgets when they are replaced by a new
  decoration of the same type. The default implementation just
  returns `false`, which will cause new instances of the widget to
  always be redrawn.
  */
  eq(widget,) {
    return false;
  }
  /**
  Update a DOM element created by a widget of the same type (but
  different, non-`eq` content) to reflect this widget. May return
  true to indicate that it could update, false to indicate it
  couldn't (in which case the widget will be redrawn). The default
  implementation just returns false.
  */
  updateDOM(dom, view,) {
    return false;
  }
  /**
  @internal
  */
  compare(other,) {
    return this == other || this.constructor == other.constructor && this.eq(other,);
  }
  /**
  The estimated height this widget will have, to be used when
  estimating the height of content that hasn't been drawn. May
  return -1 to indicate you don't know. The default implementation
  returns -1.
  */
  get estimatedHeight() {
    return -1;
  }
  /**
  For inline widgets that are displayed inline (as opposed to
  `inline-block`) and introduce line breaks (through `<br>` tags
  or textual newlines), this must indicate the amount of line
  breaks they introduce. Defaults to 0.
  */
  get lineBreaks() {
    return 0;
  }
  /**
  Can be used to configure which kinds of events inside the widget
  should be ignored by the editor. The default is to ignore all
  events.
  */
  ignoreEvent(event,) {
    return true;
  }
  /**
  Override the way screen coordinates for positions at/in the
  widget are found. `pos` will be the offset into the widget, and
  `side` the side of the position that is being queried—less than
  zero for before, greater than zero for after, and zero for
  directly at that position.
  */
  coordsAt(dom, pos, side,) {
    return null;
  }
  /**
  @internal
  */
  get customView() {
    return null;
  }
  /**
  @internal
  */
  get isHidden() {
    return false;
  }
  /**
  This is called when the an instance of the widget is removed
  from the editor view.
  */
  destroy(dom,) {
  }
};
var BlockType = /* @__PURE__ */ function (BlockType2,) {
  BlockType2[BlockType2['Text'] = 0] = 'Text';
  BlockType2[BlockType2['WidgetBefore'] = 1] = 'WidgetBefore';
  BlockType2[BlockType2['WidgetAfter'] = 2] = 'WidgetAfter';
  BlockType2[BlockType2['WidgetRange'] = 3] = 'WidgetRange';
  return BlockType2;
}(BlockType || (BlockType = {}),);
var Decoration = class extends RangeValue {
  /**
  @internal
  */
  get heightRelevant() {
    return false;
  }
  /**
  Create a mark decoration, which influences the styling of the
  content in its range. Nested mark decorations will cause nested
  DOM elements to be created. Nesting order is determined by
  precedence of the [facet](https://codemirror.net/6/docs/ref/#view.EditorView^decorations), with
  the higher-precedence decorations creating the inner DOM nodes.
  Such elements are split on line boundaries and on the boundaries
  of lower-precedence decorations.
  */
  static mark(spec,) {
    return new MarkDecoration(spec,);
  }
  /**
  Create a widget decoration, which displays a DOM element at the
  given position.
  */
  static widget(spec,) {
    let side = Math.max(-1e4, Math.min(1e4, spec.side || 0,),), block = !!spec.block;
    side += block ? side > 0 ? 3e8 : -4e8 : side > 0 ? 1e8 : -1e8;
    return new PointDecoration(spec, side, side, block, spec.widget || null, false,);
  }
  /**
  Create a replace decoration which replaces the given range with
  a widget, or simply hides it.
  */
  static replace(spec,) {
    let block = !!spec.block, startSide, endSide;
    if (spec.isBlockGap) {
      startSide = -5e8;
      endSide = 4e8;
    } else {
      let { start, end, } = getInclusive(spec, block,);
      startSide = (start ? block ? -3e8 : -1 : 5e8) - 1;
      endSide = (end ? block ? 2e8 : 1 : -6e8) + 1;
    }
    return new PointDecoration(spec, startSide, endSide, block, spec.widget || null, true,);
  }
  /**
  Create a line decoration, which can add DOM attributes to the
  line starting at the given position.
  */
  static line(spec,) {
    return new LineDecoration(spec,);
  }
  /**
  Build a [`DecorationSet`](https://codemirror.net/6/docs/ref/#view.DecorationSet) from the given
  decorated range or ranges. If the ranges aren't already sorted,
  pass `true` for `sort` to make the library sort them for you.
  */
  static set(of, sort = false,) {
    return RangeSet.of(of, sort,);
  }
  /**
  @internal
  */
  hasHeight() {
    return this.widget ? this.widget.estimatedHeight > -1 : false;
  }
  constructor(startSide, endSide, widget, spec,) {
    super();
    this.startSide = startSide;
    this.endSide = endSide;
    this.widget = widget;
    this.spec = spec;
  }
};
Decoration.none = RangeSet.empty;
var MarkDecoration = class extends Decoration {
  eq(other,) {
    return this == other ||
      other instanceof MarkDecoration && this.tagName == other.tagName && this.class == other.class && attrsEq(this.attrs, other.attrs,);
  }
  range(from, to = from,) {
    if (from >= to) throw new RangeError('Mark decorations may not be empty',);
    return super.range(from, to,);
  }
  constructor(spec,) {
    let { start, end, } = getInclusive(spec,);
    super(start ? -1 : 5e8, end ? 1 : -6e8, null, spec,);
    this.tagName = spec.tagName || 'span';
    this.class = spec.class || '';
    this.attrs = spec.attributes || null;
  }
};
MarkDecoration.prototype.point = false;
var LineDecoration = class extends Decoration {
  eq(other,) {
    return other instanceof LineDecoration && this.spec.class == other.spec.class && attrsEq(this.spec.attributes, other.spec.attributes,);
  }
  range(from, to = from,) {
    if (to != from) throw new RangeError('Line decoration ranges must be zero-length',);
    return super.range(from, to,);
  }
  constructor(spec,) {
    super(-2e8, -2e8, null, spec,);
  }
};
LineDecoration.prototype.mapMode = MapMode.TrackBefore;
LineDecoration.prototype.point = true;
var PointDecoration = class extends Decoration {
  // Only relevant when this.block == true
  get type() {
    return this.startSide < this.endSide ? BlockType.WidgetRange : this.startSide <= 0 ? BlockType.WidgetBefore : BlockType.WidgetAfter;
  }
  get heightRelevant() {
    return this.block || !!this.widget && (this.widget.estimatedHeight >= 5 || this.widget.lineBreaks > 0);
  }
  eq(other,) {
    return other instanceof PointDecoration && widgetsEq(this.widget, other.widget,) && this.block == other.block &&
      this.startSide == other.startSide && this.endSide == other.endSide;
  }
  range(from, to = from,) {
    if (this.isReplace && (from > to || from == to && this.startSide > 0 && this.endSide <= 0)) {
      throw new RangeError('Invalid range for replacement decoration',);
    }
    if (!this.isReplace && to != from) throw new RangeError('Widget decorations can only have zero-length ranges',);
    return super.range(from, to,);
  }
  constructor(spec, startSide, endSide, block, widget, isReplace,) {
    super(startSide, endSide, widget, spec,);
    this.block = block;
    this.isReplace = isReplace;
    this.mapMode = !block ? MapMode.TrackDel : startSide <= 0 ? MapMode.TrackBefore : MapMode.TrackAfter;
  }
};
PointDecoration.prototype.point = true;
function getInclusive(spec, block = false,) {
  let { inclusiveStart: start, inclusiveEnd: end, } = spec;
  if (start == null) start = spec.inclusive;
  if (end == null) end = spec.inclusive;
  return { start: start !== null && start !== void 0 ? start : block, end: end !== null && end !== void 0 ? end : block, };
}
function widgetsEq(a, b,) {
  return a == b || !!(a && b && a.compare(b,));
}
function addRange(from, to, ranges, margin = 0,) {
  let last = ranges.length - 1;
  if (last >= 0 && ranges[last] + margin >= from) ranges[last] = Math.max(ranges[last], to,);
  else ranges.push(from, to,);
}
var LineView = class extends ContentView {
  // Consumes source
  merge(from, to, source, hasStart, openStart, openEnd,) {
    if (source) {
      if (!(source instanceof LineView)) return false;
      if (!this.dom) source.transferDOM(this,);
    }
    if (hasStart) this.setDeco(source ? source.attrs : null,);
    mergeChildrenInto(this, from, to, source ? source.children : [], openStart, openEnd,);
    return true;
  }
  split(at,) {
    let end = new LineView();
    end.breakAfter = this.breakAfter;
    if (this.length == 0) return end;
    let { i: i2, off, } = this.childPos(at,);
    if (off) {
      end.append(this.children[i2].split(off,), 0,);
      this.children[i2].merge(off, this.children[i2].length, null, false, 0, 0,);
      i2++;
    }
    for (let j = i2; j < this.children.length; j++) end.append(this.children[j], 0,);
    while (i2 > 0 && this.children[i2 - 1].length == 0) this.children[--i2].destroy();
    this.children.length = i2;
    this.markDirty();
    this.length = at;
    return end;
  }
  transferDOM(other,) {
    if (!this.dom) return;
    this.markDirty();
    other.setDOM(this.dom,);
    other.prevAttrs = this.prevAttrs === void 0 ? this.attrs : this.prevAttrs;
    this.prevAttrs = void 0;
    this.dom = null;
  }
  setDeco(attrs,) {
    if (!attrsEq(this.attrs, attrs,)) {
      if (this.dom) {
        this.prevAttrs = this.attrs;
        this.markDirty();
      }
      this.attrs = attrs;
    }
  }
  append(child, openStart,) {
    joinInlineInto(this, child, openStart,);
  }
  // Only called when building a line view in ContentBuilder
  addLineDeco(deco,) {
    let attrs = deco.spec.attributes, cls = deco.spec.class;
    if (attrs) this.attrs = combineAttrs(attrs, this.attrs || {},);
    if (cls) this.attrs = combineAttrs({ class: cls, }, this.attrs || {},);
  }
  domAtPos(pos,) {
    return inlineDOMAtPos(this, pos,);
  }
  reuseDOM(node,) {
    if (node.nodeName == 'DIV') {
      this.setDOM(node,);
      this.dirty |= 4 | 2;
    }
  }
  sync(view, track,) {
    var _a2;
    if (!this.dom) {
      this.setDOM(document.createElement('div',),);
      this.dom.className = 'cm-line';
      this.prevAttrs = this.attrs ? null : void 0;
    } else if (this.dirty & 4) {
      clearAttributes(this.dom,);
      this.dom.className = 'cm-line';
      this.prevAttrs = this.attrs ? null : void 0;
    }
    if (this.prevAttrs !== void 0) {
      updateAttrs(this.dom, this.prevAttrs, this.attrs,);
      this.dom.classList.add('cm-line',);
      this.prevAttrs = void 0;
    }
    super.sync(view, track,);
    let last = this.dom.lastChild;
    while (last && ContentView.get(last,) instanceof MarkView) last = last.lastChild;
    if (
      !last || !this.length ||
      last.nodeName != 'BR' && ((_a2 = ContentView.get(last,)) === null || _a2 === void 0 ? void 0 : _a2.isEditable) == false &&
        (!browser.ios || !this.children.some((ch,) => ch instanceof TextView))
    ) {
      let hack = document.createElement('BR',);
      hack.cmIgnore = true;
      this.dom.appendChild(hack,);
    }
  }
  measureTextSize() {
    if (this.children.length == 0 || this.length > 20) return null;
    let totalWidth = 0, textHeight;
    for (let child of this.children) {
      if (!(child instanceof TextView) || /[^ -~]/.test(child.text,)) return null;
      let rects = clientRectsFor(child.dom,);
      if (rects.length != 1) return null;
      totalWidth += rects[0].width;
      textHeight = rects[0].height;
    }
    return !totalWidth ? null : { lineHeight: this.dom.getBoundingClientRect().height, charWidth: totalWidth / this.length, textHeight, };
  }
  coordsAt(pos, side,) {
    let rect = coordsInChildren(this, pos, side,);
    if (!this.children.length && rect && this.parent) {
      let { heightOracle, } = this.parent.view.viewState, height = rect.bottom - rect.top;
      if (Math.abs(height - heightOracle.lineHeight,) < 2 && heightOracle.textHeight < height) {
        let dist = (height - heightOracle.textHeight) / 2;
        return { top: rect.top + dist, bottom: rect.bottom - dist, left: rect.left, right: rect.left, };
      }
    }
    return rect;
  }
  become(_other,) {
    return false;
  }
  get type() {
    return BlockType.Text;
  }
  static find(docView, pos,) {
    for (let i2 = 0, off = 0; i2 < docView.children.length; i2++) {
      let block = docView.children[i2], end = off + block.length;
      if (end >= pos) {
        if (block instanceof LineView) return block;
        if (end > pos) break;
      }
      off = end + block.breakAfter;
    }
    return null;
  }
  constructor() {
    super(...arguments,);
    this.children = [];
    this.length = 0;
    this.prevAttrs = void 0;
    this.attrs = null;
    this.breakAfter = 0;
  }
};
var BlockWidgetView = class extends ContentView {
  merge(from, to, source, _takeDeco, openStart, openEnd,) {
    if (
      source &&
      (!(source instanceof BlockWidgetView) || !this.widget.compare(source.widget,) || from > 0 && openStart <= 0 ||
        to < this.length && openEnd <= 0)
    ) return false;
    this.length = from + (source ? source.length : 0) + (this.length - to);
    return true;
  }
  domAtPos(pos,) {
    return pos == 0 ? DOMPos.before(this.dom,) : DOMPos.after(this.dom, pos == this.length,);
  }
  split(at,) {
    let len = this.length - at;
    this.length = at;
    let end = new BlockWidgetView(this.widget, len, this.type,);
    end.breakAfter = this.breakAfter;
    return end;
  }
  get children() {
    return noChildren;
  }
  sync(view,) {
    if (!this.dom || !this.widget.updateDOM(this.dom, view,)) {
      if (this.dom && this.prevWidget) this.prevWidget.destroy(this.dom,);
      this.prevWidget = null;
      this.setDOM(this.widget.toDOM(view,),);
      this.dom.contentEditable = 'false';
    }
  }
  get overrideDOMText() {
    return this.parent ? this.parent.view.state.doc.slice(this.posAtStart, this.posAtEnd,) : Text.empty;
  }
  domBoundsAround() {
    return null;
  }
  become(other,) {
    if (other instanceof BlockWidgetView && other.widget.constructor == this.widget.constructor) {
      if (!other.widget.compare(this.widget,)) this.markDirty(true,);
      if (this.dom && !this.prevWidget) this.prevWidget = this.widget;
      this.widget = other.widget;
      this.length = other.length;
      this.type = other.type;
      this.breakAfter = other.breakAfter;
      return true;
    }
    return false;
  }
  ignoreMutation() {
    return true;
  }
  ignoreEvent(event,) {
    return this.widget.ignoreEvent(event,);
  }
  get isEditable() {
    return false;
  }
  get isWidget() {
    return true;
  }
  coordsAt(pos, side,) {
    return this.widget.coordsAt(this.dom, pos, side,);
  }
  destroy() {
    super.destroy();
    if (this.dom) this.widget.destroy(this.dom,);
  }
  constructor(widget, length, type,) {
    super();
    this.widget = widget;
    this.length = length;
    this.type = type;
    this.breakAfter = 0;
    this.prevWidget = null;
  }
};
var ContentBuilder = class {
  posCovered() {
    if (this.content.length == 0) return !this.breakAtStart && this.doc.lineAt(this.pos,).from != this.pos;
    let last = this.content[this.content.length - 1];
    return !last.breakAfter && !(last instanceof BlockWidgetView && last.type == BlockType.WidgetBefore);
  }
  getLine() {
    if (!this.curLine) {
      this.content.push(this.curLine = new LineView(),);
      this.atCursorPos = true;
    }
    return this.curLine;
  }
  flushBuffer(active = this.bufferMarks,) {
    if (this.pendingBuffer) {
      this.curLine.append(wrapMarks(new WidgetBufferView(-1,), active,), active.length,);
      this.pendingBuffer = 0;
    }
  }
  addBlockWidget(view,) {
    this.flushBuffer();
    this.curLine = null;
    this.content.push(view,);
  }
  finish(openEnd,) {
    if (this.pendingBuffer && openEnd <= this.bufferMarks.length) this.flushBuffer();
    else this.pendingBuffer = 0;
    if (!this.posCovered()) this.getLine();
  }
  buildText(length, active, openStart,) {
    while (length > 0) {
      if (this.textOff == this.text.length) {
        let { value, lineBreak, done, } = this.cursor.next(this.skip,);
        this.skip = 0;
        if (done) throw new Error('Ran out of text content when drawing inline views',);
        if (lineBreak) {
          if (!this.posCovered()) this.getLine();
          if (this.content.length) this.content[this.content.length - 1].breakAfter = 1;
          else this.breakAtStart = 1;
          this.flushBuffer();
          this.curLine = null;
          this.atCursorPos = true;
          length--;
          continue;
        } else {
          this.text = value;
          this.textOff = 0;
        }
      }
      let take = Math.min(this.text.length - this.textOff, length, 512,);
      this.flushBuffer(active.slice(active.length - openStart,),);
      this.getLine().append(wrapMarks(new TextView(this.text.slice(this.textOff, this.textOff + take,),), active,), openStart,);
      this.atCursorPos = true;
      this.textOff += take;
      length -= take;
      openStart = 0;
    }
  }
  span(from, to, active, openStart,) {
    this.buildText(to - from, active, openStart,);
    this.pos = to;
    if (this.openStart < 0) this.openStart = openStart;
  }
  point(from, to, deco, active, openStart, index,) {
    if (this.disallowBlockEffectsFor[index] && deco instanceof PointDecoration) {
      if (deco.block) throw new RangeError('Block decorations may not be specified via plugins',);
      if (to > this.doc.lineAt(this.pos,).to) {
        throw new RangeError('Decorations that replace line breaks may not be specified via plugins',);
      }
    }
    let len = to - from;
    if (deco instanceof PointDecoration) {
      if (deco.block) {
        let { type, } = deco;
        if (type == BlockType.WidgetAfter && !this.posCovered()) this.getLine();
        this.addBlockWidget(new BlockWidgetView(deco.widget || new NullWidget('div',), len, type,),);
      } else {
        let view = WidgetView.create(deco.widget || new NullWidget('span',), len, len ? 0 : deco.startSide,);
        let cursorBefore = this.atCursorPos && !view.isEditable && openStart <= active.length && (from < to || deco.startSide > 0);
        let cursorAfter = !view.isEditable && (from < to || openStart > active.length || deco.startSide <= 0);
        let line = this.getLine();
        if (this.pendingBuffer == 2 && !cursorBefore && !view.isEditable) this.pendingBuffer = 0;
        this.flushBuffer(active,);
        if (cursorBefore) {
          line.append(wrapMarks(new WidgetBufferView(1,), active,), openStart,);
          openStart = active.length + Math.max(0, openStart - active.length,);
        }
        line.append(wrapMarks(view, active,), openStart,);
        this.atCursorPos = cursorAfter;
        this.pendingBuffer = !cursorAfter ? 0 : from < to || openStart > active.length ? 1 : 2;
        if (this.pendingBuffer) this.bufferMarks = active.slice();
      }
    } else if (this.doc.lineAt(this.pos,).from == this.pos) {
      this.getLine().addLineDeco(deco,);
    }
    if (len) {
      if (this.textOff + len <= this.text.length) {
        this.textOff += len;
      } else {
        this.skip += len - (this.text.length - this.textOff);
        this.text = '';
        this.textOff = 0;
      }
      this.pos = to;
    }
    if (this.openStart < 0) this.openStart = openStart;
  }
  static build(text, from, to, decorations2, dynamicDecorationMap,) {
    let builder = new ContentBuilder(text, from, to, dynamicDecorationMap,);
    builder.openEnd = RangeSet.spans(decorations2, from, to, builder,);
    if (builder.openStart < 0) builder.openStart = builder.openEnd;
    builder.finish(builder.openEnd,);
    return builder;
  }
  constructor(doc2, pos, end, disallowBlockEffectsFor,) {
    this.doc = doc2;
    this.pos = pos;
    this.end = end;
    this.disallowBlockEffectsFor = disallowBlockEffectsFor;
    this.content = [];
    this.curLine = null;
    this.breakAtStart = 0;
    this.pendingBuffer = 0;
    this.bufferMarks = [];
    this.atCursorPos = true;
    this.openStart = -1;
    this.openEnd = -1;
    this.text = '';
    this.textOff = 0;
    this.cursor = doc2.iter();
    this.skip = pos;
  }
};
function wrapMarks(view, active,) {
  for (let mark of active) view = new MarkView(mark, [view,], view.length,);
  return view;
}
var NullWidget = class extends WidgetType {
  eq(other,) {
    return other.tag == this.tag;
  }
  toDOM() {
    return document.createElement(this.tag,);
  }
  updateDOM(elt,) {
    return elt.nodeName.toLowerCase() == this.tag;
  }
  get isHidden() {
    return true;
  }
  constructor(tag,) {
    super();
    this.tag = tag;
  }
};
var clickAddsSelectionRange = /* @__PURE__ */ Facet.define();
var dragMovesSelection$1 = /* @__PURE__ */ Facet.define();
var mouseSelectionStyle = /* @__PURE__ */ Facet.define();
var exceptionSink = /* @__PURE__ */ Facet.define();
var updateListener = /* @__PURE__ */ Facet.define();
var inputHandler = /* @__PURE__ */ Facet.define();
var focusChangeEffect = /* @__PURE__ */ Facet.define();
var perLineTextDirection = /* @__PURE__ */ Facet.define({ combine: (values,) => values.some((x,) => x), },);
var nativeSelectionHidden = /* @__PURE__ */ Facet.define({ combine: (values,) => values.some((x,) => x), },);
var ScrollTarget = class {
  map(changes,) {
    return changes.empty ? this : new ScrollTarget(this.range.map(changes,), this.y, this.x, this.yMargin, this.xMargin,);
  }
  constructor(range, y = 'nearest', x = 'nearest', yMargin = 5, xMargin = 5,) {
    this.range = range;
    this.y = y;
    this.x = x;
    this.yMargin = yMargin;
    this.xMargin = xMargin;
  }
};
var scrollIntoView = /* @__PURE__ */ StateEffect.define({ map: (t2, ch,) => t2.map(ch,), },);
function logException(state, exception, context,) {
  let handler = state.facet(exceptionSink,);
  if (handler.length) handler[0](exception,);
  else if (window.onerror) window.onerror(String(exception,), context, void 0, void 0, exception,);
  else if (context) console.error(context + ':', exception,);
  else console.error(exception,);
}
var editable = /* @__PURE__ */ Facet.define({ combine: (values,) => values.length ? values[0] : true, },);
var nextPluginID = 0;
var viewPlugin = /* @__PURE__ */ Facet.define();
var ViewPlugin = class {
  /**
  Define a plugin from a constructor function that creates the
  plugin's value, given an editor view.
  */
  static define(create, spec,) {
    const { eventHandlers, provide, decorations: deco, } = spec || {};
    return new ViewPlugin(nextPluginID++, create, eventHandlers, (plugin2,) => {
      let ext = [viewPlugin.of(plugin2,),];
      if (deco) {
        ext.push(decorations.of((view,) => {
          let pluginInst = view.plugin(plugin2,);
          return pluginInst ? deco(pluginInst,) : Decoration.none;
        },),);
      }
      if (provide) ext.push(provide(plugin2,),);
      return ext;
    },);
  }
  /**
  Create a plugin for a class whose constructor takes a single
  editor view as argument.
  */
  static fromClass(cls, spec,) {
    return ViewPlugin.define((view,) => new cls(view,), spec,);
  }
  constructor(id2, create, domEventHandlers, buildExtensions,) {
    this.id = id2;
    this.create = create;
    this.domEventHandlers = domEventHandlers;
    this.extension = buildExtensions(this,);
  }
};
var PluginInstance = class {
  update(view,) {
    if (!this.value) {
      if (this.spec) {
        try {
          this.value = this.spec.create(view,);
        } catch (e) {
          logException(view.state, e, 'CodeMirror plugin crashed',);
          this.deactivate();
        }
      }
    } else if (this.mustUpdate) {
      let update = this.mustUpdate;
      this.mustUpdate = null;
      if (this.value.update) {
        try {
          this.value.update(update,);
        } catch (e1) {
          logException(update.state, e1, 'CodeMirror plugin crashed',);
          if (this.value.destroy) {
            try {
              this.value.destroy();
            } catch (_) {
            }
          }
          this.deactivate();
        }
      }
    }
    return this;
  }
  destroy(view,) {
    var _a2;
    if ((_a2 = this.value) === null || _a2 === void 0 ? void 0 : _a2.destroy) {
      try {
        this.value.destroy();
      } catch (e) {
        logException(view.state, e, 'CodeMirror plugin crashed',);
      }
    }
  }
  deactivate() {
    this.spec = this.value = null;
  }
  constructor(spec,) {
    this.spec = spec;
    this.mustUpdate = null;
    this.value = null;
  }
};
var editorAttributes = /* @__PURE__ */ Facet.define();
var contentAttributes = /* @__PURE__ */ Facet.define();
var decorations = /* @__PURE__ */ Facet.define();
var atomicRanges = /* @__PURE__ */ Facet.define();
var scrollMargins = /* @__PURE__ */ Facet.define();
function getScrollMargins(view,) {
  let left = 0, right = 0, top22 = 0, bottom = 0;
  for (let source of view.state.facet(scrollMargins,)) {
    let m = source(view,);
    if (m) {
      if (m.left != null) left = Math.max(left, m.left,);
      if (m.right != null) right = Math.max(right, m.right,);
      if (m.top != null) top22 = Math.max(top22, m.top,);
      if (m.bottom != null) bottom = Math.max(bottom, m.bottom,);
    }
  }
  return { left, right, top: top22, bottom, };
}
var styleModule = /* @__PURE__ */ Facet.define();
var ChangedRange = class {
  join(other,) {
    return new ChangedRange(
      Math.min(this.fromA, other.fromA,),
      Math.max(this.toA, other.toA,),
      Math.min(this.fromB, other.fromB,),
      Math.max(this.toB, other.toB,),
    );
  }
  addToSet(set,) {
    let i2 = set.length, me = this;
    for (; i2 > 0; i2--) {
      let range = set[i2 - 1];
      if (range.fromA > me.toA) continue;
      if (range.toA < me.fromA) break;
      me = me.join(range,);
      set.splice(i2 - 1, 1,);
    }
    set.splice(i2, 0, me,);
    return set;
  }
  static extendWithRanges(diff, ranges,) {
    if (ranges.length == 0) return diff;
    let result = [];
    for (let dI = 0, rI = 0, posA = 0, posB = 0;; dI++) {
      let next = dI == diff.length ? null : diff[dI], off = posA - posB;
      let end = next ? next.fromB : 1e9;
      while (rI < ranges.length && ranges[rI] < end) {
        let from = ranges[rI], to = ranges[rI + 1];
        let fromB = Math.max(posB, from,), toB = Math.min(end, to,);
        if (fromB <= toB) new ChangedRange(fromB + off, toB + off, fromB, toB,).addToSet(result,);
        if (to > end) break;
        else rI += 2;
      }
      if (!next) return result;
      new ChangedRange(next.fromA, next.toA, next.fromB, next.toB,).addToSet(result,);
      posA = next.toA;
      posB = next.toB;
    }
  }
  constructor(fromA, toA, fromB, toB,) {
    this.fromA = fromA;
    this.toA = toA;
    this.fromB = fromB;
    this.toB = toB;
  }
};
var ViewUpdate = class {
  /**
  @internal
  */
  static create(view, state, transactions,) {
    return new ViewUpdate(view, state, transactions,);
  }
  /**
  Tells you whether the [viewport](https://codemirror.net/6/docs/ref/#view.EditorView.viewport) or
  [visible ranges](https://codemirror.net/6/docs/ref/#view.EditorView.visibleRanges) changed in this
  update.
  */
  get viewportChanged() {
    return (this.flags & 4) > 0;
  }
  /**
  Indicates whether the height of a block element in the editor
  changed in this update.
  */
  get heightChanged() {
    return (this.flags & 2) > 0;
  }
  /**
  Returns true when the document was modified or the size of the
  editor, or elements within the editor, changed.
  */
  get geometryChanged() {
    return this.docChanged || (this.flags & (8 | 2)) > 0;
  }
  /**
  True when this update indicates a focus change.
  */
  get focusChanged() {
    return (this.flags & 1) > 0;
  }
  /**
  Whether the document changed in this update.
  */
  get docChanged() {
    return !this.changes.empty;
  }
  /**
  Whether the selection was explicitly set in this update.
  */
  get selectionSet() {
    return this.transactions.some((tr,) => tr.selection);
  }
  /**
  @internal
  */
  get empty() {
    return this.flags == 0 && this.transactions.length == 0;
  }
  constructor(view, state, transactions,) {
    this.view = view;
    this.state = state;
    this.transactions = transactions;
    this.flags = 0;
    this.startState = view.state;
    this.changes = ChangeSet.empty(this.startState.doc.length,);
    for (let tr of transactions) this.changes = this.changes.compose(tr.changes,);
    let changedRanges = [];
    this.changes.iterChangedRanges((fromA, toA, fromB, toB,) => changedRanges.push(new ChangedRange(fromA, toA, fromB, toB,),));
    this.changedRanges = changedRanges;
  }
};
var Direction = /* @__PURE__ */ function (Direction2,) {
  Direction2[Direction2['LTR'] = 0] = 'LTR';
  Direction2[Direction2['RTL'] = 1] = 'RTL';
  return Direction2;
}(Direction || (Direction = {}),);
var LTR = Direction.LTR;
var RTL = Direction.RTL;
function dec(str,) {
  let result = [];
  for (let i2 = 0; i2 < str.length; i2++) result.push(1 << +str[i2],);
  return result;
}
var LowTypes = /* @__PURE__ */ dec(
  '88888888888888888888888888888888888666888888787833333333337888888000000000000000000000000008888880000000000000000000000000088888888888888888888888888888888888887866668888088888663380888308888800000000000000000000000800000000000000000000000000000008',
);
var ArabicTypes = /* @__PURE__ */ dec(
  '4444448826627288999999999992222222222222222222222222222222222222222222222229999999999999999999994444444444644222822222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222999999949999999229989999223333333333',
);
var Brackets = /* @__PURE__ */ Object.create(null,);
var BracketStack = [];
for (let p of ['()', '[]', '{}',]) {
  let l = /* @__PURE__ */ p.charCodeAt(0,), r = /* @__PURE__ */ p.charCodeAt(1,);
  Brackets[l] = r;
  Brackets[r] = -l;
}
function charType(ch,) {
  return ch <= 247
    ? LowTypes[ch]
    : 1424 <= ch && ch <= 1524
    ? 2
    : 1536 <= ch && ch <= 1785
    ? ArabicTypes[ch - 1536]
    : 1774 <= ch && ch <= 2220
    ? 4
    : 8192 <= ch && ch <= 8203
    ? 256
    : 64336 <= ch && ch <= 65023
    ? 4
    : ch == 8204
    ? 256
    : 1;
}
var BidiRE = /[\u0590-\u05f4\u0600-\u06ff\u0700-\u08ac\ufb50-\ufdff]/;
var BidiSpan = class {
  /**
  The direction of this span.
  */
  get dir() {
    return this.level % 2 ? RTL : LTR;
  }
  /**
  @internal
  */
  side(end, dir,) {
    return this.dir == dir == end ? this.to : this.from;
  }
  /**
  @internal
  */
  static find(order, index, level, assoc,) {
    let maybe = -1;
    for (let i2 = 0; i2 < order.length; i2++) {
      let span = order[i2];
      if (span.from <= index && span.to >= index) {
        if (span.level == level) return i2;
        if (maybe < 0 || (assoc != 0 ? assoc < 0 ? span.from < index : span.to > index : order[maybe].level > span.level)) maybe = i2;
      }
    }
    if (maybe < 0) throw new RangeError('Index out of range',);
    return maybe;
  }
  /**
  @internal
  */
  constructor(from, to, level,) {
    this.from = from;
    this.to = to;
    this.level = level;
  }
};
var types = [];
function computeOrder(line, direction,) {
  let len = line.length, outerType = direction == LTR ? 1 : 2, oppositeType = direction == LTR ? 2 : 1;
  if (!line || outerType == 1 && !BidiRE.test(line,)) return trivialOrder(len,);
  for (let i2 = 0, prev = outerType, prevStrong = outerType; i2 < len; i2++) {
    let type = charType(line.charCodeAt(i2,),);
    if (type == 512) type = prev;
    else if (type == 8 && prevStrong == 4) type = 16;
    types[i2] = type == 4 ? 2 : type;
    if (type & 7) prevStrong = type;
    prev = type;
  }
  for (let i1 = 0, prev1 = outerType, prevStrong1 = outerType; i1 < len; i1++) {
    let type1 = types[i1];
    if (type1 == 128) {
      if (i1 < len - 1 && prev1 == types[i1 + 1] && prev1 & 24) type1 = types[i1] = prev1;
      else types[i1] = 256;
    } else if (type1 == 64) {
      let end = i1 + 1;
      while (end < len && types[end] == 64) end++;
      let replace = i1 && prev1 == 8 || end < len && types[end] == 8 ? prevStrong1 == 1 ? 1 : 8 : 256;
      for (let j = i1; j < end; j++) types[j] = replace;
      i1 = end - 1;
    } else if (type1 == 8 && prevStrong1 == 1) {
      types[i1] = 1;
    }
    prev1 = type1;
    if (type1 & 7) prevStrong1 = type1;
  }
  for (let i2 = 0, sI = 0, context = 0, ch, br, type2; i2 < len; i2++) {
    if (br = Brackets[ch = line.charCodeAt(i2,)]) {
      if (br < 0) {
        for (let sJ = sI - 3; sJ >= 0; sJ -= 3) {
          if (BracketStack[sJ + 1] == -br) {
            let flags = BracketStack[sJ + 2];
            let type21 = flags & 2 ? outerType : !(flags & 4) ? 0 : flags & 1 ? oppositeType : outerType;
            if (type21) types[i2] = types[BracketStack[sJ]] = type21;
            sI = sJ;
            break;
          }
        }
      } else if (BracketStack.length == 189) {
        break;
      } else {
        BracketStack[sI++] = i2;
        BracketStack[sI++] = ch;
        BracketStack[sI++] = context;
      }
    } else if ((type2 = types[i2]) == 2 || type2 == 1) {
      let embed = type2 == outerType;
      context = embed ? 0 : 1;
      for (let sJ1 = sI - 3; sJ1 >= 0; sJ1 -= 3) {
        let cur = BracketStack[sJ1 + 2];
        if (cur & 2) break;
        if (embed) {
          BracketStack[sJ1 + 2] |= 2;
        } else {
          if (cur & 4) break;
          BracketStack[sJ1 + 2] |= 4;
        }
      }
    }
  }
  for (let i3 = 0; i3 < len; i3++) {
    if (types[i3] == 256) {
      let end1 = i3 + 1;
      while (end1 < len && types[end1] == 256) end1++;
      let beforeL = (i3 ? types[i3 - 1] : outerType) == 1;
      let afterL = (end1 < len ? types[end1] : outerType) == 1;
      let replace1 = beforeL == afterL ? beforeL ? 1 : 2 : outerType;
      for (let j1 = i3; j1 < end1; j1++) types[j1] = replace1;
      i3 = end1 - 1;
    }
  }
  let order = [];
  if (outerType == 1) {
    for (let i4 = 0; i4 < len;) {
      let start = i4, rtl = types[i4++] != 1;
      while (i4 < len && rtl == (types[i4] != 1)) i4++;
      if (rtl) {
        for (let j2 = i4; j2 > start;) {
          let end2 = j2, l = types[--j2] != 2;
          while (j2 > start && l == (types[j2 - 1] != 2)) j2--;
          order.push(new BidiSpan(j2, end2, l ? 2 : 1,),);
        }
      } else {
        order.push(new BidiSpan(start, i4, 0,),);
      }
    }
  } else {
    for (let i5 = 0; i5 < len;) {
      let start1 = i5, rtl1 = types[i5++] == 2;
      while (i5 < len && rtl1 == (types[i5] == 2)) i5++;
      order.push(new BidiSpan(start1, i5, rtl1 ? 1 : 2,),);
    }
  }
  return order;
}
function trivialOrder(length,) {
  return [new BidiSpan(0, length, 0,),];
}
var movedOver = '';
function moveVisually(line, order, dir, start, forward,) {
  var _a2;
  let startIndex = start.head - line.from, spanI = -1;
  if (startIndex == 0) {
    if (!forward || !line.length) return null;
    if (order[0].level != dir) {
      startIndex = order[0].side(false, dir,);
      spanI = 0;
    }
  } else if (startIndex == line.length) {
    if (forward) return null;
    let last = order[order.length - 1];
    if (last.level != dir) {
      startIndex = last.side(true, dir,);
      spanI = order.length - 1;
    }
  }
  if (spanI < 0) spanI = BidiSpan.find(order, startIndex, (_a2 = start.bidiLevel) !== null && _a2 !== void 0 ? _a2 : -1, start.assoc,);
  let span = order[spanI];
  if (startIndex == span.side(forward, dir,)) {
    span = order[spanI += forward ? 1 : -1];
    startIndex = span.side(!forward, dir,);
  }
  let indexForward = forward == (span.dir == dir);
  let nextIndex = findClusterBreak(line.text, startIndex, indexForward,);
  movedOver = line.text.slice(Math.min(startIndex, nextIndex,), Math.max(startIndex, nextIndex,),);
  if (nextIndex != span.side(forward, dir,)) return EditorSelection.cursor(nextIndex + line.from, indexForward ? -1 : 1, span.level,);
  let nextSpan = spanI == (forward ? order.length - 1 : 0) ? null : order[spanI + (forward ? 1 : -1)];
  if (!nextSpan && span.level != dir) return EditorSelection.cursor(forward ? line.to : line.from, forward ? -1 : 1, dir,);
  if (nextSpan && nextSpan.level < span.level) {
    return EditorSelection.cursor(nextSpan.side(!forward, dir,) + line.from, forward ? 1 : -1, nextSpan.level,);
  }
  return EditorSelection.cursor(nextIndex + line.from, forward ? -1 : 1, span.level,);
}
var LineBreakPlaceholder = '\uFFFF';
var DOMReader = class {
  append(text,) {
    this.text += text;
  }
  lineBreak() {
    this.text += LineBreakPlaceholder;
  }
  readRange(start, end,) {
    if (!start) return this;
    let parent = start.parentNode;
    for (let cur = start;;) {
      this.findPointBefore(parent, cur,);
      let oldLen = this.text.length;
      this.readNode(cur,);
      let next = cur.nextSibling;
      if (next == end) break;
      let view = ContentView.get(cur,), nextView = ContentView.get(next,);
      if (
        view && nextView
          ? view.breakAfter
          : (view ? view.breakAfter : isBlockElement(cur,)) ||
            isBlockElement(next,) && (cur.nodeName != 'BR' || cur.cmIgnore) && this.text.length > oldLen
      ) this.lineBreak();
      cur = next;
    }
    this.findPointBefore(parent, end,);
    return this;
  }
  readTextNode(node,) {
    let text = node.nodeValue;
    for (let point of this.points) if (point.node == node) point.pos = this.text.length + Math.min(point.offset, text.length,);
    for (let off = 0, re = this.lineSeparator ? null : /\r\n?|\n/g;;) {
      let nextBreak = -1, breakSize = 1, m;
      if (this.lineSeparator) {
        nextBreak = text.indexOf(this.lineSeparator, off,);
        breakSize = this.lineSeparator.length;
      } else if (m = re.exec(text,)) {
        nextBreak = m.index;
        breakSize = m[0].length;
      }
      this.append(text.slice(off, nextBreak < 0 ? text.length : nextBreak,),);
      if (nextBreak < 0) break;
      this.lineBreak();
      if (breakSize > 1) {
        for (let point1 of this.points) if (point1.node == node && point1.pos > this.text.length) point1.pos -= breakSize - 1;
      }
      off = nextBreak + breakSize;
    }
  }
  readNode(node,) {
    if (node.cmIgnore) return;
    let view = ContentView.get(node,);
    let fromView = view && view.overrideDOMText;
    if (fromView != null) {
      this.findPointInside(node, fromView.length,);
      for (let i2 = fromView.iter(); !i2.next().done;) {
        if (i2.lineBreak) this.lineBreak();
        else this.append(i2.value,);
      }
    } else if (node.nodeType == 3) {
      this.readTextNode(node,);
    } else if (node.nodeName == 'BR') {
      if (node.nextSibling) this.lineBreak();
    } else if (node.nodeType == 1) {
      this.readRange(node.firstChild, null,);
    }
  }
  findPointBefore(node, next,) {
    for (let point of this.points) if (point.node == node && node.childNodes[point.offset] == next) point.pos = this.text.length;
  }
  findPointInside(node, maxLen,) {
    for (let point of this.points) {
      if (node.nodeType == 3 ? point.node == node : node.contains(point.node,)) {
        point.pos = this.text.length + Math.min(maxLen, point.offset,);
      }
    }
  }
  constructor(points, state,) {
    this.points = points;
    this.text = '';
    this.lineSeparator = state.facet(EditorState.lineSeparator,);
  }
};
function isBlockElement(node,) {
  return node.nodeType == 1 && /^(DIV|P|LI|UL|OL|BLOCKQUOTE|DD|DT|H\d|SECTION|PRE)$/.test(node.nodeName,);
}
var DOMPoint = class {
  constructor(node, offset,) {
    this.node = node;
    this.offset = offset;
    this.pos = -1;
  }
};
var DocView = class extends ContentView {
  get length() {
    return this.view.state.doc.length;
  }
  // Update the document view to a given state.
  update(update,) {
    let changedRanges = update.changedRanges;
    if (this.minWidth > 0 && changedRanges.length) {
      if (!changedRanges.every(({ fromA, toA, },) => toA < this.minWidthFrom || fromA > this.minWidthTo)) {
        this.minWidth = this.minWidthFrom = this.minWidthTo = 0;
      } else {
        this.minWidthFrom = update.changes.mapPos(this.minWidthFrom, 1,);
        this.minWidthTo = update.changes.mapPos(this.minWidthTo, 1,);
      }
    }
    if (this.view.inputState.composing < 0) this.compositionDeco = Decoration.none;
    else if (update.transactions.length || this.dirty) this.compositionDeco = computeCompositionDeco(this.view, update.changes,);
    if ((browser.ie || browser.chrome) && !this.compositionDeco.size && update && update.state.doc.lines != update.startState.doc.lines) {
      this.forceSelection = true;
    }
    let prevDeco = this.decorations, deco = this.updateDeco();
    let decoDiff = findChangedDeco(prevDeco, deco, update.changes,);
    changedRanges = ChangedRange.extendWithRanges(changedRanges, decoDiff,);
    if (this.dirty == 0 && changedRanges.length == 0) {
      return false;
    } else {
      this.updateInner(changedRanges, update.startState.doc.length,);
      if (update.transactions.length) this.lastUpdate = Date.now();
      return true;
    }
  }
  // Used by update and the constructor do perform the actual DOM
  // update
  updateInner(changes, oldLength,) {
    this.view.viewState.mustMeasureContent = true;
    this.updateChildren(changes, oldLength,);
    let { observer, } = this.view;
    observer.ignore(() => {
      this.dom.style.height = this.view.viewState.contentHeight + 'px';
      this.dom.style.flexBasis = this.minWidth ? this.minWidth + 'px' : '';
      let track = browser.chrome || browser.ios ? { node: observer.selectionRange.focusNode, written: false, } : void 0;
      this.sync(this.view, track,);
      this.dirty = 0;
      if (track && (track.written || observer.selectionRange.focusNode != track.node)) this.forceSelection = true;
      this.dom.style.height = '';
    },);
    let gaps = [];
    if (this.view.viewport.from || this.view.viewport.to < this.view.state.doc.length) {
      for (let child of this.children) {
        if (child instanceof BlockWidgetView && child.widget instanceof BlockGapWidget) gaps.push(child.dom,);
      }
    }
    observer.updateGaps(gaps,);
  }
  updateChildren(changes, oldLength,) {
    let cursor = this.childCursor(oldLength,);
    for (let i2 = changes.length - 1;; i2--) {
      let next = i2 >= 0 ? changes[i2] : null;
      if (!next) break;
      let { fromA, toA, fromB, toB, } = next;
      let { content: content2, breakAtStart, openStart, openEnd, } = ContentBuilder.build(
        this.view.state.doc,
        fromB,
        toB,
        this.decorations,
        this.dynamicDecorationMap,
      );
      let { i: toI, off: toOff, } = cursor.findPos(toA, 1,);
      let { i: fromI, off: fromOff, } = cursor.findPos(fromA, -1,);
      replaceRange(this, fromI, fromOff, toI, toOff, content2, breakAtStart, openStart, openEnd,);
    }
  }
  // Sync the DOM selection to this.state.selection
  updateSelection(mustRead = false, fromPointer = false,) {
    if (mustRead || !this.view.observer.selectionRange.focusNode) this.view.observer.readSelectionRange();
    let activeElt = this.view.root.activeElement, focused = activeElt == this.dom;
    let selectionNotFocus = !focused && hasSelection(this.dom, this.view.observer.selectionRange,) &&
      !(activeElt && this.dom.contains(activeElt,));
    if (!(focused || fromPointer || selectionNotFocus)) return;
    let force = this.forceSelection;
    this.forceSelection = false;
    let main = this.view.state.selection.main;
    let anchor = this.domAtPos(main.anchor,);
    let head = main.empty ? anchor : this.domAtPos(main.head,);
    if (browser.gecko && main.empty && !this.compositionDeco.size && betweenUneditable(anchor,)) {
      let dummy = document.createTextNode('',);
      this.view.observer.ignore(() => anchor.node.insertBefore(dummy, anchor.node.childNodes[anchor.offset] || null,));
      anchor = head = new DOMPos(dummy, 0,);
      force = true;
    }
    let domSel = this.view.observer.selectionRange;
    if (
      force || !domSel.focusNode || !isEquivalentPosition(anchor.node, anchor.offset, domSel.anchorNode, domSel.anchorOffset,) ||
      !isEquivalentPosition(head.node, head.offset, domSel.focusNode, domSel.focusOffset,)
    ) {
      this.view.observer.ignore(() => {
        if (browser.android && browser.chrome && this.dom.contains(domSel.focusNode,) && inUneditable(domSel.focusNode, this.dom,)) {
          this.dom.blur();
          this.dom.focus({ preventScroll: true, },);
        }
        let rawSel = getSelection(this.view.root,);
        if (!rawSel);
        else if (main.empty) {
          if (browser.gecko) {
            let nextTo = nextToUneditable(anchor.node, anchor.offset,);
            if (nextTo && nextTo != (1 | 2)) {
              let text = nearbyTextNode(anchor.node, anchor.offset, nextTo == 1 ? 1 : -1,);
              if (text) anchor = new DOMPos(text, nextTo == 1 ? 0 : text.nodeValue.length,);
            }
          }
          rawSel.collapse(anchor.node, anchor.offset,);
          if (main.bidiLevel != null && domSel.cursorBidiLevel != null) domSel.cursorBidiLevel = main.bidiLevel;
        } else if (rawSel.extend) {
          rawSel.collapse(anchor.node, anchor.offset,);
          try {
            rawSel.extend(head.node, head.offset,);
          } catch (_) {
          }
        } else {
          let range = document.createRange();
          if (main.anchor > main.head) [anchor, head,] = [head, anchor,];
          range.setEnd(head.node, head.offset,);
          range.setStart(anchor.node, anchor.offset,);
          rawSel.removeAllRanges();
          rawSel.addRange(range,);
        }
        if (selectionNotFocus && this.view.root.activeElement == this.dom) {
          this.dom.blur();
          if (activeElt) activeElt.focus();
        }
      },);
      this.view.observer.setSelectionRange(anchor, head,);
    }
    this.impreciseAnchor = anchor.precise ? null : new DOMPos(domSel.anchorNode, domSel.anchorOffset,);
    this.impreciseHead = head.precise ? null : new DOMPos(domSel.focusNode, domSel.focusOffset,);
  }
  enforceCursorAssoc() {
    if (this.compositionDeco.size) return;
    let { view, } = this, cursor = view.state.selection.main;
    let sel = getSelection(view.root,);
    let { anchorNode, anchorOffset, } = view.observer.selectionRange;
    if (!sel || !cursor.empty || !cursor.assoc || !sel.modify) return;
    let line = LineView.find(this, cursor.head,);
    if (!line) return;
    let lineStart = line.posAtStart;
    if (cursor.head == lineStart || cursor.head == lineStart + line.length) return;
    let before = this.coordsAt(cursor.head, -1,), after = this.coordsAt(cursor.head, 1,);
    if (!before || !after || before.bottom > after.top) return;
    let dom = this.domAtPos(cursor.head + cursor.assoc,);
    sel.collapse(dom.node, dom.offset,);
    sel.modify('move', cursor.assoc < 0 ? 'forward' : 'backward', 'lineboundary',);
    view.observer.readSelectionRange();
    let newRange = view.observer.selectionRange;
    if (view.docView.posFromDOM(newRange.anchorNode, newRange.anchorOffset,) != cursor.from) sel.collapse(anchorNode, anchorOffset,);
  }
  nearest(dom,) {
    for (let cur = dom; cur;) {
      let domView = ContentView.get(cur,);
      if (domView && domView.rootView == this) return domView;
      cur = cur.parentNode;
    }
    return null;
  }
  posFromDOM(node, offset,) {
    let view = this.nearest(node,);
    if (!view) throw new RangeError('Trying to find position for a DOM position outside of the document',);
    return view.localPosFromDOM(node, offset,) + view.posAtStart;
  }
  domAtPos(pos,) {
    let { i: i2, off, } = this.childCursor().findPos(pos, -1,);
    for (; i2 < this.children.length - 1;) {
      let child = this.children[i2];
      if (off < child.length || child instanceof LineView) break;
      i2++;
      off = 0;
    }
    return this.children[i2].domAtPos(off,);
  }
  coordsAt(pos, side,) {
    for (let off = this.length, i2 = this.children.length - 1;; i2--) {
      let child = this.children[i2], start = off - child.breakAfter - child.length;
      if (
        pos > start ||
        pos == start && child.type != BlockType.WidgetBefore && child.type != BlockType.WidgetAfter &&
          (!i2 || side == 2 || this.children[i2 - 1].breakAfter || this.children[i2 - 1].type == BlockType.WidgetBefore && side > -2)
      ) return child.coordsAt(pos - start, side,);
      off = start;
    }
  }
  measureVisibleLineHeights(viewport,) {
    let result = [], { from, to, } = viewport;
    let contentWidth = this.view.contentDOM.clientWidth;
    let isWider = contentWidth > Math.max(this.view.scrollDOM.clientWidth, this.minWidth,) + 1;
    let widest = -1, ltr = this.view.textDirection == Direction.LTR;
    for (let pos = 0, i2 = 0; i2 < this.children.length; i2++) {
      let child = this.children[i2], end = pos + child.length;
      if (end > to) break;
      if (pos >= from) {
        let childRect = child.dom.getBoundingClientRect();
        result.push(childRect.height,);
        if (isWider) {
          let last = child.dom.lastChild;
          let rects = last ? clientRectsFor(last,) : [];
          if (rects.length) {
            let rect = rects[rects.length - 1];
            let width = ltr ? rect.right - childRect.left : childRect.right - rect.left;
            if (width > widest) {
              widest = width;
              this.minWidth = contentWidth;
              this.minWidthFrom = pos;
              this.minWidthTo = end;
            }
          }
        }
      }
      pos = end + child.breakAfter;
    }
    return result;
  }
  textDirectionAt(pos,) {
    let { i: i2, } = this.childPos(pos, 1,);
    return getComputedStyle(this.children[i2].dom,).direction == 'rtl' ? Direction.RTL : Direction.LTR;
  }
  measureTextSize() {
    for (let child of this.children) {
      if (child instanceof LineView) {
        let measure = child.measureTextSize();
        if (measure) return measure;
      }
    }
    let dummy = document.createElement('div',), lineHeight, charWidth, textHeight;
    dummy.className = 'cm-line';
    dummy.style.width = '99999px';
    dummy.textContent = 'abc def ghi jkl mno pqr stu';
    this.view.observer.ignore(() => {
      this.dom.appendChild(dummy,);
      let rect = clientRectsFor(dummy.firstChild,)[0];
      lineHeight = dummy.getBoundingClientRect().height;
      charWidth = rect ? rect.width / 27 : 7;
      textHeight = rect ? rect.height : lineHeight;
      dummy.remove();
    },);
    return { lineHeight, charWidth, textHeight, };
  }
  childCursor(pos = this.length,) {
    let i2 = this.children.length;
    if (i2) pos -= this.children[--i2].length;
    return new ChildCursor(this.children, pos, i2,);
  }
  computeBlockGapDeco() {
    let deco = [], vs = this.view.viewState;
    for (let pos = 0, i2 = 0;; i2++) {
      let next = i2 == vs.viewports.length ? null : vs.viewports[i2];
      let end = next ? next.from - 1 : this.length;
      if (end > pos) {
        let height = vs.lineBlockAt(end,).bottom - vs.lineBlockAt(pos,).top;
        deco.push(
          Decoration.replace({ widget: new BlockGapWidget(height,), block: true, inclusive: true, isBlockGap: true, },).range(pos, end,),
        );
      }
      if (!next) break;
      pos = next.to + 1;
    }
    return Decoration.set(deco,);
  }
  updateDeco() {
    let allDeco = this.view.state.facet(decorations,).map((d, i2,) => {
      let dynamic = this.dynamicDecorationMap[i2] = typeof d == 'function';
      return dynamic ? d(this.view,) : d;
    },);
    for (let i2 = allDeco.length; i2 < allDeco.length + 3; i2++) this.dynamicDecorationMap[i2] = false;
    return this.decorations = [...allDeco, this.compositionDeco, this.computeBlockGapDeco(), this.view.viewState.lineGapDeco,];
  }
  scrollIntoView(target,) {
    let { range, } = target;
    let rect = this.coordsAt(range.head, range.empty ? range.assoc : range.head > range.anchor ? -1 : 1,), other;
    if (!rect) return;
    if (!range.empty && (other = this.coordsAt(range.anchor, range.anchor > range.head ? -1 : 1,))) {
      rect = {
        left: Math.min(rect.left, other.left,),
        top: Math.min(rect.top, other.top,),
        right: Math.max(rect.right, other.right,),
        bottom: Math.max(rect.bottom, other.bottom,),
      };
    }
    let margins = getScrollMargins(this.view,);
    let targetRect = {
      left: rect.left - margins.left,
      top: rect.top - margins.top,
      right: rect.right + margins.right,
      bottom: rect.bottom + margins.bottom,
    };
    scrollRectIntoView(
      this.view.scrollDOM,
      targetRect,
      range.head < range.anchor ? -1 : 1,
      target.x,
      target.y,
      target.xMargin,
      target.yMargin,
      this.view.textDirection == Direction.LTR,
    );
  }
  constructor(view,) {
    super();
    this.view = view;
    this.compositionDeco = Decoration.none;
    this.decorations = [];
    this.dynamicDecorationMap = [];
    this.minWidth = 0;
    this.minWidthFrom = 0;
    this.minWidthTo = 0;
    this.impreciseAnchor = null;
    this.impreciseHead = null;
    this.forceSelection = false;
    this.lastUpdate = Date.now();
    this.setDOM(view.contentDOM,);
    this.children = [new LineView(),];
    this.children[0].setParent(this,);
    this.updateDeco();
    this.updateInner([new ChangedRange(0, 0, 0, view.state.doc.length,),], 0,);
  }
};
function betweenUneditable(pos,) {
  return pos.node.nodeType == 1 && pos.node.firstChild &&
    (pos.offset == 0 || pos.node.childNodes[pos.offset - 1].contentEditable == 'false') &&
    (pos.offset == pos.node.childNodes.length || pos.node.childNodes[pos.offset].contentEditable == 'false');
}
var BlockGapWidget = class extends WidgetType {
  toDOM() {
    let elt = document.createElement('div',);
    this.updateDOM(elt,);
    return elt;
  }
  eq(other,) {
    return other.height == this.height;
  }
  updateDOM(elt,) {
    elt.style.height = this.height + 'px';
    return true;
  }
  get estimatedHeight() {
    return this.height;
  }
  constructor(height,) {
    super();
    this.height = height;
  }
};
function compositionSurroundingNode(view,) {
  let sel = view.observer.selectionRange;
  let textNode = sel.focusNode && nearbyTextNode(sel.focusNode, sel.focusOffset, 0,);
  if (!textNode) return null;
  let cView = view.docView.nearest(textNode,);
  if (!cView) return null;
  if (cView instanceof LineView) {
    let topNode = textNode;
    while (topNode.parentNode != cView.dom) topNode = topNode.parentNode;
    let prev = topNode.previousSibling;
    while (prev && !ContentView.get(prev,)) prev = prev.previousSibling;
    let pos = prev ? ContentView.get(prev,).posAtEnd : cView.posAtStart;
    return { from: pos, to: pos, node: topNode, text: textNode, };
  } else {
    for (;;) {
      let { parent, } = cView;
      if (!parent) return null;
      if (parent instanceof LineView) break;
      cView = parent;
    }
    let from = cView.posAtStart;
    return { from, to: from + cView.length, node: cView.dom, text: textNode, };
  }
}
function computeCompositionDeco(view, changes,) {
  let surrounding = compositionSurroundingNode(view,);
  if (!surrounding) return Decoration.none;
  let { from, to, node, text: textNode, } = surrounding;
  let newFrom = changes.mapPos(from, 1,), newTo = Math.max(newFrom, changes.mapPos(to, -1,),);
  let { state, } = view, reader = new DOMReader([], state,);
  if (node.nodeType == 3) reader.readTextNode(node,);
  else reader.readRange(node.firstChild, null,);
  let { text, } = reader;
  if (text.indexOf(LineBreakPlaceholder,) > -1) return Decoration.none;
  if (newTo - newFrom < text.length) {
    if (state.doc.sliceString(newFrom, Math.min(state.doc.length, newFrom + text.length,),) == text) newTo = newFrom + text.length;
    else if (state.doc.sliceString(Math.max(0, newTo - text.length,), newTo,) == text) newFrom = newTo - text.length;
    else return Decoration.none;
  } else if (state.doc.sliceString(newFrom, newTo,) != text) {
    return Decoration.none;
  }
  let topView = ContentView.get(node,);
  if (topView instanceof CompositionView) topView = topView.widget.topView;
  else if (topView) topView.parent = null;
  return Decoration.set(
    Decoration.replace({ widget: new CompositionWidget(node, textNode, topView,), inclusive: true, },).range(newFrom, newTo,),
  );
}
var CompositionWidget = class extends WidgetType {
  eq(other,) {
    return this.top == other.top && this.text == other.text;
  }
  toDOM() {
    return this.top;
  }
  ignoreEvent() {
    return false;
  }
  get customView() {
    return CompositionView;
  }
  constructor(top22, text, topView,) {
    super();
    this.top = top22;
    this.text = text;
    this.topView = topView;
  }
};
function nearbyTextNode(startNode, startOffset, side,) {
  if (side <= 0) {
    for (let node = startNode, offset = startOffset;;) {
      if (node.nodeType == 3) return node;
      if (node.nodeType == 1 && offset > 0) {
        node = node.childNodes[offset - 1];
        offset = maxOffset(node,);
      } else {
        break;
      }
    }
  }
  if (side >= 0) {
    for (let node1 = startNode, offset1 = startOffset;;) {
      if (node1.nodeType == 3) return node1;
      if (node1.nodeType == 1 && offset1 < node1.childNodes.length && side >= 0) {
        node1 = node1.childNodes[offset1];
        offset1 = 0;
      } else {
        break;
      }
    }
  }
  return null;
}
function nextToUneditable(node, offset,) {
  if (node.nodeType != 1) return 0;
  return (offset && node.childNodes[offset - 1].contentEditable == 'false' ? 1 : 0) |
    (offset < node.childNodes.length && node.childNodes[offset].contentEditable == 'false' ? 2 : 0);
}
var DecorationComparator$1 = class {
  compareRange(from, to,) {
    addRange(from, to, this.changes,);
  }
  comparePoint(from, to,) {
    addRange(from, to, this.changes,);
  }
  constructor() {
    this.changes = [];
  }
};
function findChangedDeco(a, b, diff,) {
  let comp = new DecorationComparator$1();
  RangeSet.compare(a, b, diff, comp,);
  return comp.changes;
}
function inUneditable(node, inside2,) {
  for (let cur = node; cur && cur != inside2; cur = cur.assignedSlot || cur.parentNode) {
    if (cur.nodeType == 1 && cur.contentEditable == 'false') {
      return true;
    }
  }
  return false;
}
function groupAt(state, pos, bias = 1,) {
  let categorize = state.charCategorizer(pos,);
  let line = state.doc.lineAt(pos,), linePos = pos - line.from;
  if (line.length == 0) return EditorSelection.cursor(pos,);
  if (linePos == 0) bias = 1;
  else if (linePos == line.length) bias = -1;
  let from = linePos, to = linePos;
  if (bias < 0) from = findClusterBreak(line.text, linePos, false,);
  else to = findClusterBreak(line.text, linePos,);
  let cat = categorize(line.text.slice(from, to,),);
  while (from > 0) {
    let prev = findClusterBreak(line.text, from, false,);
    if (categorize(line.text.slice(prev, from,),) != cat) break;
    from = prev;
  }
  while (to < line.length) {
    let next = findClusterBreak(line.text, to,);
    if (categorize(line.text.slice(to, next,),) != cat) break;
    to = next;
  }
  return EditorSelection.range(from + line.from, to + line.from,);
}
function getdx(x, rect,) {
  return rect.left > x ? rect.left - x : Math.max(0, x - rect.right,);
}
function getdy(y, rect,) {
  return rect.top > y ? rect.top - y : Math.max(0, y - rect.bottom,);
}
function yOverlap(a, b,) {
  return a.top < b.bottom - 1 && a.bottom > b.top + 1;
}
function upTop(rect, top22,) {
  return top22 < rect.top ? { top: top22, left: rect.left, right: rect.right, bottom: rect.bottom, } : rect;
}
function upBot(rect, bottom,) {
  return bottom > rect.bottom ? { top: rect.top, left: rect.left, right: rect.right, bottom, } : rect;
}
function domPosAtCoords(parent, x, y,) {
  let closest, closestRect, closestX, closestY, closestOverlap = false;
  let above, below, aboveRect, belowRect;
  for (let child = parent.firstChild; child; child = child.nextSibling) {
    let rects = clientRectsFor(child,);
    for (let i2 = 0; i2 < rects.length; i2++) {
      let rect = rects[i2];
      if (closestRect && yOverlap(closestRect, rect,)) rect = upTop(upBot(rect, closestRect.bottom,), closestRect.top,);
      let dx = getdx(x, rect,), dy = getdy(y, rect,);
      if (dx == 0 && dy == 0) return child.nodeType == 3 ? domPosInText(child, x, y,) : domPosAtCoords(child, x, y,);
      if (!closest || closestY > dy || closestY == dy && closestX > dx) {
        closest = child;
        closestRect = rect;
        closestX = dx;
        closestY = dy;
        let side = dy ? y < rect.top ? -1 : 1 : dx ? x < rect.left ? -1 : 1 : 0;
        closestOverlap = !side || (side > 0 ? i2 < rects.length - 1 : i2 > 0);
      }
      if (dx == 0) {
        if (y > rect.bottom && (!aboveRect || aboveRect.bottom < rect.bottom)) {
          above = child;
          aboveRect = rect;
        } else if (y < rect.top && (!belowRect || belowRect.top > rect.top)) {
          below = child;
          belowRect = rect;
        }
      } else if (aboveRect && yOverlap(aboveRect, rect,)) {
        aboveRect = upBot(aboveRect, rect.bottom,);
      } else if (belowRect && yOverlap(belowRect, rect,)) {
        belowRect = upTop(belowRect, rect.top,);
      }
    }
  }
  if (aboveRect && aboveRect.bottom >= y) {
    closest = above;
    closestRect = aboveRect;
  } else if (belowRect && belowRect.top <= y) {
    closest = below;
    closestRect = belowRect;
  }
  if (!closest) return { node: parent, offset: 0, };
  let clipX = Math.max(closestRect.left, Math.min(closestRect.right, x,),);
  if (closest.nodeType == 3) return domPosInText(closest, clipX, y,);
  if (closestOverlap && closest.contentEditable != 'false') return domPosAtCoords(closest, clipX, y,);
  let offset = Array.prototype.indexOf.call(parent.childNodes, closest,) + (x >= (closestRect.left + closestRect.right) / 2 ? 1 : 0);
  return { node: parent, offset, };
}
function domPosInText(node, x, y,) {
  let len = node.nodeValue.length;
  let closestOffset = -1, closestDY = 1e9, generalSide = 0;
  for (let i2 = 0; i2 < len; i2++) {
    let rects = textRange(node, i2, i2 + 1,).getClientRects();
    for (let j = 0; j < rects.length; j++) {
      let rect = rects[j];
      if (rect.top == rect.bottom) continue;
      if (!generalSide) generalSide = x - rect.left;
      let dy = (rect.top > y ? rect.top - y : y - rect.bottom) - 1;
      if (rect.left - 1 <= x && rect.right + 1 >= x && dy < closestDY) {
        let right = x >= (rect.left + rect.right) / 2, after = right;
        if (browser.chrome || browser.gecko) {
          let rectBefore = textRange(node, i2,).getBoundingClientRect();
          if (rectBefore.left == rect.right) after = !right;
        }
        if (dy <= 0) return { node, offset: i2 + (after ? 1 : 0), };
        closestOffset = i2 + (after ? 1 : 0);
        closestDY = dy;
      }
    }
  }
  return { node, offset: closestOffset > -1 ? closestOffset : generalSide > 0 ? node.nodeValue.length : 0, };
}
function posAtCoords(view, coords, precise, bias = -1,) {
  var _a2, _b;
  let content2 = view.contentDOM.getBoundingClientRect(), docTop = content2.top + view.viewState.paddingTop;
  let block, { docHeight, } = view.viewState;
  let { x, y, } = coords, yOffset = y - docTop;
  if (yOffset < 0) return 0;
  if (yOffset > docHeight) return view.state.doc.length;
  for (let halfLine = view.defaultLineHeight / 2, bounced = false;;) {
    block = view.elementAtHeight(yOffset,);
    if (block.type == BlockType.Text) break;
    for (;;) {
      yOffset = bias > 0 ? block.bottom + halfLine : block.top - halfLine;
      if (yOffset >= 0 && yOffset <= docHeight) break;
      if (bounced) return precise ? null : 0;
      bounced = true;
      bias = -bias;
    }
  }
  y = docTop + yOffset;
  let lineStart = block.from;
  if (lineStart < view.viewport.from) {
    return view.viewport.from == 0
      ? 0
      : precise
      ? null
      : posAtCoordsImprecise(view, content2, block, x, y,);
  }
  if (lineStart > view.viewport.to) {
    return view.viewport.to == view.state.doc.length
      ? view.state.doc.length
      : precise
      ? null
      : posAtCoordsImprecise(view, content2, block, x, y,);
  }
  let doc2 = view.dom.ownerDocument;
  let root = view.root.elementFromPoint ? view.root : doc2;
  let element = root.elementFromPoint(x, y,);
  if (element && !view.contentDOM.contains(element,)) element = null;
  if (!element) {
    x = Math.max(content2.left + 1, Math.min(content2.right - 1, x,),);
    element = root.elementFromPoint(x, y,);
    if (element && !view.contentDOM.contains(element,)) element = null;
  }
  let node, offset = -1;
  if (element && ((_a2 = view.docView.nearest(element,)) === null || _a2 === void 0 ? void 0 : _a2.isEditable) != false) {
    if (doc2.caretPositionFromPoint) {
      let pos = doc2.caretPositionFromPoint(x, y,);
      if (pos) ({ offsetNode: node, offset, } = pos);
    } else if (doc2.caretRangeFromPoint) {
      let range = doc2.caretRangeFromPoint(x, y,);
      if (range) {
        ({ startContainer: node, startOffset: offset, } = range);
        if (
          !view.contentDOM.contains(node,) || browser.safari && isSuspiciousSafariCaretResult(node, offset, x,) ||
          browser.chrome && isSuspiciousChromeCaretResult(node, offset, x,)
        ) node = void 0;
      }
    }
  }
  if (!node || !view.docView.dom.contains(node,)) {
    let line = LineView.find(view.docView, lineStart,);
    if (!line) return yOffset > block.top + block.height / 2 ? block.to : block.from;
    ({ node, offset, } = domPosAtCoords(line.dom, x, y,));
  }
  let nearest = view.docView.nearest(node,);
  if (!nearest) return null;
  if (nearest.isWidget && ((_b = nearest.dom) === null || _b === void 0 ? void 0 : _b.nodeType) == 1) {
    let rect = nearest.dom.getBoundingClientRect();
    return coords.y < rect.top || coords.y <= rect.bottom && coords.x <= (rect.left + rect.right) / 2
      ? nearest.posAtStart
      : nearest.posAtEnd;
  } else {
    return nearest.localPosFromDOM(node, offset,) + nearest.posAtStart;
  }
}
function posAtCoordsImprecise(view, contentRect, block, x, y,) {
  let into = Math.round((x - contentRect.left) * view.defaultCharacterWidth,);
  if (view.lineWrapping && block.height > view.defaultLineHeight * 1.5) {
    let line = Math.floor((y - block.top) / view.defaultLineHeight,);
    into += line * view.viewState.heightOracle.lineLength;
  }
  let content2 = view.state.sliceDoc(block.from, block.to,);
  return block.from + findColumn(content2, into, view.state.tabSize,);
}
function isSuspiciousSafariCaretResult(node, offset, x,) {
  let len;
  if (node.nodeType != 3 || offset != (len = node.nodeValue.length)) return false;
  for (let next = node.nextSibling; next; next = next.nextSibling) if (next.nodeType != 1 || next.nodeName != 'BR') return false;
  return textRange(node, len - 1, len,).getBoundingClientRect().left > x;
}
function isSuspiciousChromeCaretResult(node, offset, x,) {
  if (offset != 0) return false;
  for (let cur = node;;) {
    let parent = cur.parentNode;
    if (!parent || parent.nodeType != 1 || parent.firstChild != cur) return false;
    if (parent.classList.contains('cm-line',)) break;
    cur = parent;
  }
  let rect = node.nodeType == 1
    ? node.getBoundingClientRect()
    : textRange(node, 0, Math.max(node.nodeValue.length, 1,),).getBoundingClientRect();
  return x - rect.left > 5;
}
function blockAt(view, pos,) {
  let line = view.lineBlockAt(pos,);
  if (Array.isArray(line.type,)) {
    for (let l of line.type) {
      if (l.to > pos || l.to == pos && (l.to == line.to || l.type == BlockType.Text)) return l;
    }
  }
  return line;
}
function moveToLineBoundary(view, start, forward, includeWrap,) {
  let line = blockAt(view, start.head,);
  let coords = !includeWrap || line.type != BlockType.Text || !(view.lineWrapping || line.widgetLineBreaks)
    ? null
    : view.coordsAtPos(start.assoc < 0 && start.head > line.from ? start.head - 1 : start.head,);
  if (coords) {
    let editorRect = view.dom.getBoundingClientRect();
    let direction = view.textDirectionAt(line.from,);
    let pos = view.posAtCoords({
      x: forward == (direction == Direction.LTR) ? editorRect.right - 1 : editorRect.left + 1,
      y: (coords.top + coords.bottom) / 2,
    },);
    if (pos != null) return EditorSelection.cursor(pos, forward ? -1 : 1,);
  }
  return EditorSelection.cursor(forward ? line.to : line.from, forward ? -1 : 1,);
}
function moveByChar(view, start, forward, by,) {
  let line = view.state.doc.lineAt(start.head,), spans = view.bidiSpans(line,);
  let direction = view.textDirectionAt(line.from,);
  for (let cur = start, check = null;;) {
    let next = moveVisually(line, spans, direction, cur, forward,), char = movedOver;
    if (!next) {
      if (line.number == (forward ? view.state.doc.lines : 1)) return cur;
      char = '\n';
      line = view.state.doc.line(line.number + (forward ? 1 : -1),);
      spans = view.bidiSpans(line,);
      next = EditorSelection.cursor(forward ? line.from : line.to,);
    }
    if (!check) {
      if (!by) return next;
      check = by(char,);
    } else if (!check(char,)) {
      return cur;
    }
    cur = next;
  }
}
function byGroup(view, pos, start,) {
  let categorize = view.state.charCategorizer(pos,);
  let cat = categorize(start,);
  return (next,) => {
    let nextCat = categorize(next,);
    if (cat == CharCategory.Space) cat = nextCat;
    return cat == nextCat;
  };
}
function moveVertically(view, start, forward, distance,) {
  let startPos = start.head, dir = forward ? 1 : -1;
  if (startPos == (forward ? view.state.doc.length : 0)) return EditorSelection.cursor(startPos, start.assoc,);
  let goal = start.goalColumn, startY;
  let rect = view.contentDOM.getBoundingClientRect();
  let startCoords = view.coordsAtPos(startPos,), docTop = view.documentTop;
  if (startCoords) {
    if (goal == null) goal = startCoords.left - rect.left;
    startY = dir < 0 ? startCoords.top : startCoords.bottom;
  } else {
    let line = view.viewState.lineBlockAt(startPos,);
    if (goal == null) goal = Math.min(rect.right - rect.left, view.defaultCharacterWidth * (startPos - line.from),);
    startY = (dir < 0 ? line.top : line.bottom) + docTop;
  }
  let resolvedGoal = rect.left + goal;
  let dist = distance !== null && distance !== void 0 ? distance : view.defaultLineHeight >> 1;
  for (let extra = 0;; extra += 10) {
    let curY = startY + (dist + extra) * dir;
    let pos = posAtCoords(view, { x: resolvedGoal, y: curY, }, false, dir,);
    if (curY < rect.top || curY > rect.bottom || (dir < 0 ? pos < startPos : pos > startPos)) {
      return EditorSelection.cursor(pos, start.assoc, void 0, goal,);
    }
  }
}
function skipAtomicRanges(atoms, pos, bias,) {
  for (;;) {
    let moved = 0;
    for (let set of atoms) {
      set.between(pos - 1, pos + 1, (from, to, value,) => {
        if (pos > from && pos < to) {
          let side = moved || bias || (pos - from < to - pos ? -1 : 1);
          pos = side < 0 ? from : to;
          moved = side;
        }
      },);
    }
    if (!moved) return pos;
  }
}
function skipAtoms(view, oldPos, pos,) {
  let newPos = skipAtomicRanges(view.state.facet(atomicRanges,).map((f,) => f(view,)), pos.from, oldPos.head > pos.from ? -1 : 1,);
  return newPos == pos.from ? pos : EditorSelection.cursor(newPos, newPos < pos.from ? 1 : -1,);
}
var InputState = class {
  setSelectionOrigin(origin,) {
    this.lastSelectionOrigin = origin;
    this.lastSelectionTime = Date.now();
  }
  ensureHandlers(view, plugins,) {
    var _a2;
    let handlers2;
    this.customHandlers = [];
    for (let plugin2 of plugins) {
      if (handlers2 = (_a2 = plugin2.update(view,).spec) === null || _a2 === void 0 ? void 0 : _a2.domEventHandlers) {
        this.customHandlers.push({ plugin: plugin2.value, handlers: handlers2, },);
        for (let type in handlers2) {
          if (this.registeredEvents.indexOf(type,) < 0 && type != 'scroll') {
            this.registeredEvents.push(type,);
            view.contentDOM.addEventListener(type, (event,) => {
              if (!eventBelongsToEditor(view, event,)) return;
              if (this.runCustomHandlers(type, view, event,)) event.preventDefault();
            },);
          }
        }
      }
    }
  }
  runCustomHandlers(type, view, event,) {
    for (let set of this.customHandlers) {
      let handler = set.handlers[type];
      if (handler) {
        try {
          if (handler.call(set.plugin, event, view,) || event.defaultPrevented) return true;
        } catch (e) {
          logException(view.state, e,);
        }
      }
    }
    return false;
  }
  runScrollHandlers(view, event,) {
    this.lastScrollTop = view.scrollDOM.scrollTop;
    this.lastScrollLeft = view.scrollDOM.scrollLeft;
    for (let set of this.customHandlers) {
      let handler = set.handlers.scroll;
      if (handler) {
        try {
          handler.call(set.plugin, event, view,);
        } catch (e) {
          logException(view.state, e,);
        }
      }
    }
  }
  keydown(view, event,) {
    this.lastKeyCode = event.keyCode;
    this.lastKeyTime = Date.now();
    if (event.keyCode == 9 && Date.now() < this.lastEscPress + 2e3) return true;
    if (event.keyCode != 27 && modifierCodes.indexOf(event.keyCode,) < 0) view.inputState.lastEscPress = 0;
    if (browser.android && browser.chrome && !event.synthetic && (event.keyCode == 13 || event.keyCode == 8)) {
      view.observer.delayAndroidKey(event.key, event.keyCode,);
      return true;
    }
    let pending;
    if (
      browser.ios && !event.synthetic && !event.altKey && !event.metaKey &&
      ((pending = PendingKeys.find((key,) => key.keyCode == event.keyCode)) && !event.ctrlKey ||
        EmacsyPendingKeys.indexOf(event.key,) > -1 && event.ctrlKey && !event.shiftKey)
    ) {
      this.pendingIOSKey = pending || event;
      setTimeout(() => this.flushIOSKey(view,), 250,);
      return true;
    }
    return false;
  }
  flushIOSKey(view,) {
    let key = this.pendingIOSKey;
    if (!key) return false;
    this.pendingIOSKey = void 0;
    return dispatchKey(view.contentDOM, key.key, key.keyCode,);
  }
  ignoreDuringComposition(event,) {
    if (!/^key/.test(event.type,)) return false;
    if (this.composing > 0) return true;
    if (browser.safari && !browser.ios && this.compositionPendingKey && Date.now() - this.compositionEndedAt < 100) {
      this.compositionPendingKey = false;
      return true;
    }
    return false;
  }
  mustFlushObserver(event,) {
    return event.type == 'keydown' && event.keyCode != 229;
  }
  startMouseSelection(mouseSelection,) {
    if (this.mouseSelection) this.mouseSelection.destroy();
    this.mouseSelection = mouseSelection;
  }
  update(update,) {
    if (this.mouseSelection) this.mouseSelection.update(update,);
    if (update.transactions.length) this.lastKeyCode = this.lastSelectionTime = 0;
  }
  destroy() {
    if (this.mouseSelection) this.mouseSelection.destroy();
  }
  constructor(view,) {
    this.lastKeyCode = 0;
    this.lastKeyTime = 0;
    this.lastTouchTime = 0;
    this.lastFocusTime = 0;
    this.lastScrollTop = 0;
    this.lastScrollLeft = 0;
    this.chromeScrollHack = -1;
    this.pendingIOSKey = void 0;
    this.lastSelectionOrigin = null;
    this.lastSelectionTime = 0;
    this.lastEscPress = 0;
    this.lastContextMenu = 0;
    this.scrollHandlers = [];
    this.registeredEvents = [];
    this.customHandlers = [];
    this.composing = -1;
    this.compositionFirstChange = null;
    this.compositionEndedAt = 0;
    this.compositionPendingKey = false;
    this.compositionPendingChange = false;
    this.mouseSelection = null;
    let handleEvent = (handler, event,) => {
      if (this.ignoreDuringComposition(event,)) return;
      if (event.type == 'keydown' && this.keydown(view, event,)) return;
      if (this.mustFlushObserver(event,)) view.observer.forceFlush();
      if (this.runCustomHandlers(event.type, view, event,)) event.preventDefault();
      else handler(view, event,);
    };
    for (let type in handlers) {
      let handler = handlers[type];
      view.contentDOM.addEventListener(type, (event,) => {
        if (eventBelongsToEditor(view, event,)) handleEvent(handler, event,);
      }, handlerOptions[type],);
      this.registeredEvents.push(type,);
    }
    view.scrollDOM.addEventListener('mousedown', (event,) => {
      if (event.target == view.scrollDOM && event.clientY > view.contentDOM.getBoundingClientRect().bottom) {
        handleEvent(handlers.mousedown, event,);
        if (!event.defaultPrevented && event.button == 2) {
          let start = view.contentDOM.style.minHeight;
          view.contentDOM.style.minHeight = '100%';
          setTimeout(() => view.contentDOM.style.minHeight = start, 200,);
        }
      }
    },);
    view.scrollDOM.addEventListener('drop', (event,) => {
      if (event.target == view.scrollDOM && event.clientY > view.contentDOM.getBoundingClientRect().bottom) {
        handleEvent(handlers.drop, event,);
      }
    },);
    if (browser.chrome && browser.chrome_version == 102) {
      view.scrollDOM.addEventListener('wheel', () => {
        if (this.chromeScrollHack < 0) view.contentDOM.style.pointerEvents = 'none';
        else window.clearTimeout(this.chromeScrollHack,);
        this.chromeScrollHack = setTimeout(() => {
          this.chromeScrollHack = -1;
          view.contentDOM.style.pointerEvents = '';
        }, 100,);
      }, { passive: true, },);
    }
    this.notifiedFocused = view.hasFocus;
    if (browser.safari) view.contentDOM.addEventListener('input', () => null,);
  }
};
var PendingKeys = [{ key: 'Backspace', keyCode: 8, inputType: 'deleteContentBackward', }, {
  key: 'Enter',
  keyCode: 13,
  inputType: 'insertParagraph',
}, { key: 'Delete', keyCode: 46, inputType: 'deleteContentForward', },];
var EmacsyPendingKeys = 'dthko';
var modifierCodes = [16, 17, 18, 20, 91, 92, 224, 225,];
var dragScrollMargin = 6;
function dragScrollSpeed(dist,) {
  return Math.max(0, dist,) * 0.7 + 8;
}
var MouseSelection = class {
  start(event,) {
    if (this.dragging === false) {
      event.preventDefault();
      this.select(event,);
    }
  }
  move(event,) {
    var _a2;
    if (event.buttons == 0) return this.destroy();
    if (this.dragging !== false) return;
    this.select(this.lastEvent = event,);
    let sx = 0, sy = 0;
    let rect = ((_a2 = this.scrollParent) === null || _a2 === void 0 ? void 0 : _a2.getBoundingClientRect()) ||
      { left: 0, top: 0, right: this.view.win.innerWidth, bottom: this.view.win.innerHeight, };
    let margins = getScrollMargins(this.view,);
    if (event.clientX - margins.left <= rect.left + dragScrollMargin) sx = -dragScrollSpeed(rect.left - event.clientX,);
    else if (event.clientX + margins.right >= rect.right - dragScrollMargin) sx = dragScrollSpeed(event.clientX - rect.right,);
    if (event.clientY - margins.top <= rect.top + dragScrollMargin) sy = -dragScrollSpeed(rect.top - event.clientY,);
    else if (event.clientY + margins.bottom >= rect.bottom - dragScrollMargin) sy = dragScrollSpeed(event.clientY - rect.bottom,);
    this.setScrollSpeed(sx, sy,);
  }
  up(event,) {
    if (this.dragging == null) this.select(this.lastEvent,);
    if (!this.dragging) event.preventDefault();
    this.destroy();
  }
  destroy() {
    this.setScrollSpeed(0, 0,);
    let doc2 = this.view.contentDOM.ownerDocument;
    doc2.removeEventListener('mousemove', this.move,);
    doc2.removeEventListener('mouseup', this.up,);
    this.view.inputState.mouseSelection = null;
  }
  setScrollSpeed(sx, sy,) {
    this.scrollSpeed = { x: sx, y: sy, };
    if (sx || sy) {
      if (this.scrolling < 0) this.scrolling = setInterval(() => this.scroll(), 50,);
    } else if (this.scrolling > -1) {
      clearInterval(this.scrolling,);
      this.scrolling = -1;
    }
  }
  scroll() {
    if (this.scrollParent) {
      this.scrollParent.scrollLeft += this.scrollSpeed.x;
      this.scrollParent.scrollTop += this.scrollSpeed.y;
    } else {
      this.view.win.scrollBy(this.scrollSpeed.x, this.scrollSpeed.y,);
    }
    if (this.dragging === false) this.select(this.lastEvent,);
  }
  skipAtoms(sel,) {
    let ranges = null;
    for (let i2 = 0; i2 < sel.ranges.length; i2++) {
      let range = sel.ranges[i2], updated = null;
      if (range.empty) {
        let pos = skipAtomicRanges(this.atoms, range.from, 0,);
        if (pos != range.from) updated = EditorSelection.cursor(pos, -1,);
      } else {
        let from = skipAtomicRanges(this.atoms, range.from, -1,);
        let to = skipAtomicRanges(this.atoms, range.to, 1,);
        if (from != range.from || to != range.to) {
          updated = EditorSelection.range(range.from == range.anchor ? from : to, range.from == range.head ? from : to,);
        }
      }
      if (updated) {
        if (!ranges) ranges = sel.ranges.slice();
        ranges[i2] = updated;
      }
    }
    return ranges ? EditorSelection.create(ranges, sel.mainIndex,) : sel;
  }
  select(event,) {
    let { view, } = this, selection = this.skipAtoms(this.style.get(event, this.extend, this.multiple,),);
    if (this.mustSelect || !selection.eq(view.state.selection,) || selection.main.assoc != view.state.selection.main.assoc) {
      this.view.dispatch({ selection, userEvent: 'select.pointer', },);
    }
    this.mustSelect = false;
  }
  update(update,) {
    if (update.docChanged && this.dragging) this.dragging = this.dragging.map(update.changes,);
    if (this.style.update(update,)) setTimeout(() => this.select(this.lastEvent,), 20,);
  }
  constructor(view, startEvent, style, mustSelect,) {
    this.view = view;
    this.style = style;
    this.mustSelect = mustSelect;
    this.scrollSpeed = { x: 0, y: 0, };
    this.scrolling = -1;
    this.lastEvent = startEvent;
    this.scrollParent = scrollableParent(view.contentDOM,);
    this.atoms = view.state.facet(atomicRanges,).map((f,) => f(view,));
    let doc2 = view.contentDOM.ownerDocument;
    doc2.addEventListener('mousemove', this.move = this.move.bind(this,),);
    doc2.addEventListener('mouseup', this.up = this.up.bind(this,),);
    this.extend = startEvent.shiftKey;
    this.multiple = view.state.facet(EditorState.allowMultipleSelections,) && addsSelectionRange(view, startEvent,);
    this.dragMove = dragMovesSelection(view, startEvent,);
    this.dragging = isInPrimarySelection(view, startEvent,) && getClickType(startEvent,) == 1 ? null : false;
  }
};
function addsSelectionRange(view, event,) {
  let facet = view.state.facet(clickAddsSelectionRange,);
  return facet.length ? facet[0](event,) : browser.mac ? event.metaKey : event.ctrlKey;
}
function dragMovesSelection(view, event,) {
  let facet = view.state.facet(dragMovesSelection$1,);
  return facet.length ? facet[0](event,) : browser.mac ? !event.altKey : !event.ctrlKey;
}
function isInPrimarySelection(view, event,) {
  let { main, } = view.state.selection;
  if (main.empty) return false;
  let sel = getSelection(view.root,);
  if (!sel || sel.rangeCount == 0) return true;
  let rects = sel.getRangeAt(0,).getClientRects();
  for (let i2 = 0; i2 < rects.length; i2++) {
    let rect = rects[i2];
    if (rect.left <= event.clientX && rect.right >= event.clientX && rect.top <= event.clientY && rect.bottom >= event.clientY) return true;
  }
  return false;
}
function eventBelongsToEditor(view, event,) {
  if (!event.bubbles) return true;
  if (event.defaultPrevented) return false;
  for (let node = event.target, cView; node != view.contentDOM; node = node.parentNode) {
    if (!node || node.nodeType == 11 || (cView = ContentView.get(node,)) && cView.ignoreEvent(event,)) return false;
  }
  return true;
}
var handlers = /* @__PURE__ */ Object.create(null,);
var handlerOptions = /* @__PURE__ */ Object.create(null,);
var brokenClipboardAPI = browser.ie && browser.ie_version < 15 || browser.ios && browser.webkit_version < 604;
function capturePaste(view,) {
  let parent = view.dom.parentNode;
  if (!parent) return;
  let target = parent.appendChild(document.createElement('textarea',),);
  target.style.cssText = 'position: fixed; left: -10000px; top: 10px';
  target.focus();
  setTimeout(() => {
    view.focus();
    target.remove();
    doPaste(view, target.value,);
  }, 50,);
}
function doPaste(view, input,) {
  let { state, } = view, changes, i2 = 1, text = state.toText(input,);
  let byLine = text.lines == state.selection.ranges.length;
  let linewise = lastLinewiseCopy != null && state.selection.ranges.every((r,) => r.empty) && lastLinewiseCopy == text.toString();
  if (linewise) {
    let lastLine = -1;
    changes = state.changeByRange((range,) => {
      let line = state.doc.lineAt(range.from,);
      if (line.from == lastLine) return { range, };
      lastLine = line.from;
      let insert2 = state.toText((byLine ? text.line(i2++,).text : input) + state.lineBreak,);
      return { changes: { from: line.from, insert: insert2, }, range: EditorSelection.cursor(range.from + insert2.length,), };
    },);
  } else if (byLine) {
    changes = state.changeByRange((range,) => {
      let line = text.line(i2++,);
      return { changes: { from: range.from, to: range.to, insert: line.text, }, range: EditorSelection.cursor(range.from + line.length,), };
    },);
  } else {
    changes = state.replaceSelection(text,);
  }
  view.dispatch(changes, { userEvent: 'input.paste', scrollIntoView: true, },);
}
handlers.keydown = (view, event,) => {
  view.inputState.setSelectionOrigin('select',);
  if (event.keyCode == 27) view.inputState.lastEscPress = Date.now();
};
handlers.touchstart = (view, e,) => {
  view.inputState.lastTouchTime = Date.now();
  view.inputState.setSelectionOrigin('select.pointer',);
};
handlers.touchmove = (view,) => {
  view.inputState.setSelectionOrigin('select.pointer',);
};
handlerOptions.touchstart = handlerOptions.touchmove = { passive: true, };
handlers.mousedown = (view, event,) => {
  view.observer.flush();
  if (view.inputState.lastTouchTime > Date.now() - 2e3) return;
  let style = null;
  for (let makeStyle of view.state.facet(mouseSelectionStyle,)) {
    style = makeStyle(view, event,);
    if (style) break;
  }
  if (!style && event.button == 0) style = basicMouseSelection(view, event,);
  if (style) {
    let mustFocus = view.root.activeElement != view.contentDOM;
    view.inputState.startMouseSelection(new MouseSelection(view, event, style, mustFocus,),);
    if (mustFocus) view.observer.ignore(() => focusPreventScroll(view.contentDOM,));
    if (view.inputState.mouseSelection) view.inputState.mouseSelection.start(event,);
  }
};
function rangeForClick(view, pos, bias, type,) {
  if (type == 1) {
    return EditorSelection.cursor(pos, bias,);
  } else if (type == 2) {
    return groupAt(view.state, pos, bias,);
  } else {
    let visual = LineView.find(view.docView, pos,), line = view.state.doc.lineAt(visual ? visual.posAtEnd : pos,);
    let from = visual ? visual.posAtStart : line.from, to = visual ? visual.posAtEnd : line.to;
    if (to < view.state.doc.length && to == line.to) to++;
    return EditorSelection.range(from, to,);
  }
}
var insideY = (y, rect,) => y >= rect.top && y <= rect.bottom;
var inside = (x, y, rect,) => insideY(y, rect,) && x >= rect.left && x <= rect.right;
function findPositionSide(view, pos, x, y,) {
  let line = LineView.find(view.docView, pos,);
  if (!line) return 1;
  let off = pos - line.posAtStart;
  if (off == 0) return 1;
  if (off == line.length) return -1;
  let before = line.coordsAt(off, -1,);
  if (before && inside(x, y, before,)) return -1;
  let after = line.coordsAt(off, 1,);
  if (after && inside(x, y, after,)) return 1;
  return before && insideY(y, before,) ? -1 : 1;
}
function queryPos(view, event,) {
  let pos = view.posAtCoords({ x: event.clientX, y: event.clientY, }, false,);
  return { pos, bias: findPositionSide(view, pos, event.clientX, event.clientY,), };
}
var BadMouseDetail = browser.ie && browser.ie_version <= 11;
var lastMouseDown = null;
var lastMouseDownCount = 0;
var lastMouseDownTime = 0;
function getClickType(event,) {
  if (!BadMouseDetail) return event.detail;
  let last = lastMouseDown, lastTime = lastMouseDownTime;
  lastMouseDown = event;
  lastMouseDownTime = Date.now();
  return lastMouseDownCount =
    !last || lastTime > Date.now() - 400 && Math.abs(last.clientX - event.clientX,) < 2 && Math.abs(last.clientY - event.clientY,) < 2
      ? (lastMouseDownCount + 1) % 3
      : 1;
}
function basicMouseSelection(view, event,) {
  let start = queryPos(view, event,), type = getClickType(event,);
  let startSel = view.state.selection;
  return {
    update(update,) {
      if (update.docChanged) {
        start.pos = update.changes.mapPos(start.pos,);
        startSel = startSel.map(update.changes,);
      }
    },
    get(event2, extend2, multiple,) {
      let cur = queryPos(view, event2,), removed;
      let range = rangeForClick(view, cur.pos, cur.bias, type,);
      if (start.pos != cur.pos && !extend2) {
        let startRange = rangeForClick(view, start.pos, start.bias, type,);
        let from = Math.min(startRange.from, range.from,), to = Math.max(startRange.to, range.to,);
        range = from < range.from ? EditorSelection.range(from, to,) : EditorSelection.range(to, from,);
      }
      if (extend2) return startSel.replaceRange(startSel.main.extend(range.from, range.to,),);
      else if (multiple && type == 1 && startSel.ranges.length > 1 && (removed = removeRangeAround(startSel, cur.pos,))) return removed;
      else if (multiple) return startSel.addRange(range,);
      else return EditorSelection.create([range,],);
    },
  };
}
function removeRangeAround(sel, pos,) {
  for (let i2 = 0; i2 < sel.ranges.length; i2++) {
    let { from, to, } = sel.ranges[i2];
    if (from <= pos && to >= pos) {
      return EditorSelection.create(
        sel.ranges.slice(0, i2,).concat(sel.ranges.slice(i2 + 1,),),
        sel.mainIndex == i2 ? 0 : sel.mainIndex - (sel.mainIndex > i2 ? 1 : 0),
      );
    }
  }
  return null;
}
handlers.dragstart = (view, event,) => {
  let { selection: { main, }, } = view.state;
  let { mouseSelection, } = view.inputState;
  if (mouseSelection) mouseSelection.dragging = main;
  if (event.dataTransfer) {
    event.dataTransfer.setData('Text', view.state.sliceDoc(main.from, main.to,),);
    event.dataTransfer.effectAllowed = 'copyMove';
  }
};
function dropText(view, event, text, direct,) {
  if (!text) return;
  let dropPos = view.posAtCoords({ x: event.clientX, y: event.clientY, }, false,);
  event.preventDefault();
  let { mouseSelection, } = view.inputState;
  let del = direct && mouseSelection && mouseSelection.dragging && mouseSelection.dragMove
    ? { from: mouseSelection.dragging.from, to: mouseSelection.dragging.to, }
    : null;
  let ins = { from: dropPos, insert: text, };
  let changes = view.state.changes(del ? [del, ins,] : ins,);
  view.focus();
  view.dispatch({
    changes,
    selection: { anchor: changes.mapPos(dropPos, -1,), head: changes.mapPos(dropPos, 1,), },
    userEvent: del ? 'move.drop' : 'input.drop',
  },);
}
handlers.drop = (view, event,) => {
  if (!event.dataTransfer) return;
  if (view.state.readOnly) return event.preventDefault();
  let files = event.dataTransfer.files;
  if (files && files.length) {
    event.preventDefault();
    let text = Array(files.length,), read = 0;
    let finishFile = () => {
      if (++read == files.length) dropText(view, event, text.filter((s,) => s != null).join(view.state.lineBreak,), false,);
    };
    for (let i2 = 0; i2 < files.length; i2++) {
      let reader = new FileReader();
      reader.onerror = finishFile;
      reader.onload = () => {
        if (!/[\x00-\x08\x0e-\x1f]{2}/.test(reader.result,)) text[i2] = reader.result;
        finishFile();
      };
      reader.readAsText(files[i2],);
    }
  } else {
    dropText(view, event, event.dataTransfer.getData('Text',), true,);
  }
};
handlers.paste = (view, event,) => {
  if (view.state.readOnly) return event.preventDefault();
  view.observer.flush();
  let data = brokenClipboardAPI ? null : event.clipboardData;
  if (data) {
    doPaste(view, data.getData('text/plain',) || data.getData('text/uri-text',),);
    event.preventDefault();
  } else {
    capturePaste(view,);
  }
};
function captureCopy(view, text,) {
  let parent = view.dom.parentNode;
  if (!parent) return;
  let target = parent.appendChild(document.createElement('textarea',),);
  target.style.cssText = 'position: fixed; left: -10000px; top: 10px';
  target.value = text;
  target.focus();
  target.selectionEnd = text.length;
  target.selectionStart = 0;
  setTimeout(() => {
    target.remove();
    view.focus();
  }, 50,);
}
function copiedRange(state,) {
  let content2 = [], ranges = [], linewise = false;
  for (let range of state.selection.ranges) {
    if (!range.empty) {
      content2.push(state.sliceDoc(range.from, range.to,),);
      ranges.push(range,);
    }
  }
  if (!content2.length) {
    let upto = -1;
    for (let { from, } of state.selection.ranges) {
      let line = state.doc.lineAt(from,);
      if (line.number > upto) {
        content2.push(line.text,);
        ranges.push({ from: line.from, to: Math.min(state.doc.length, line.to + 1,), },);
      }
      upto = line.number;
    }
    linewise = true;
  }
  return { text: content2.join(state.lineBreak,), ranges, linewise, };
}
var lastLinewiseCopy = null;
handlers.copy = handlers.cut = (view, event,) => {
  let { text, ranges, linewise, } = copiedRange(view.state,);
  if (!text && !linewise) return;
  lastLinewiseCopy = linewise ? text : null;
  let data = brokenClipboardAPI ? null : event.clipboardData;
  if (data) {
    event.preventDefault();
    data.clearData();
    data.setData('text/plain', text,);
  } else {
    captureCopy(view, text,);
  }
  if (event.type == 'cut' && !view.state.readOnly) view.dispatch({ changes: ranges, scrollIntoView: true, userEvent: 'delete.cut', },);
};
var isFocusChange = /* @__PURE__ */ Annotation.define();
function focusChangeTransaction(state, focus,) {
  let effects = [];
  for (let getEffect of state.facet(focusChangeEffect,)) {
    let effect = getEffect(state, focus,);
    if (effect) effects.push(effect,);
  }
  return effects ? state.update({ effects, annotations: isFocusChange.of(true,), },) : null;
}
function updateForFocusChange(view,) {
  setTimeout(() => {
    let focus = view.hasFocus;
    if (focus != view.inputState.notifiedFocused) {
      let tr = focusChangeTransaction(view.state, focus,);
      if (tr) view.dispatch(tr,);
      else view.update([],);
    }
  }, 10,);
}
handlers.focus = (view,) => {
  view.inputState.lastFocusTime = Date.now();
  if (!view.scrollDOM.scrollTop && (view.inputState.lastScrollTop || view.inputState.lastScrollLeft)) {
    view.scrollDOM.scrollTop = view.inputState.lastScrollTop;
    view.scrollDOM.scrollLeft = view.inputState.lastScrollLeft;
  }
  updateForFocusChange(view,);
};
handlers.blur = (view,) => {
  view.observer.clearSelectionRange();
  updateForFocusChange(view,);
};
handlers.compositionstart = handlers.compositionupdate = (view,) => {
  if (view.inputState.compositionFirstChange == null) view.inputState.compositionFirstChange = true;
  if (view.inputState.composing < 0) {
    view.inputState.composing = 0;
  }
};
handlers.compositionend = (view,) => {
  view.inputState.composing = -1;
  view.inputState.compositionEndedAt = Date.now();
  view.inputState.compositionPendingKey = true;
  view.inputState.compositionPendingChange = view.observer.pendingRecords().length > 0;
  view.inputState.compositionFirstChange = null;
  if (browser.chrome && browser.android) {
    view.observer.flushSoon();
  } else if (view.inputState.compositionPendingChange) {
    Promise.resolve().then(() => view.observer.flush());
  } else {
    setTimeout(() => {
      if (view.inputState.composing < 0 && view.docView.compositionDeco.size) view.update([],);
    }, 50,);
  }
};
handlers.contextmenu = (view,) => {
  view.inputState.lastContextMenu = Date.now();
};
handlers.beforeinput = (view, event,) => {
  var _a2;
  let pending;
  if (browser.chrome && browser.android && (pending = PendingKeys.find((key,) => key.inputType == event.inputType))) {
    view.observer.delayAndroidKey(pending.key, pending.keyCode,);
    if (pending.key == 'Backspace' || pending.key == 'Delete') {
      let startViewHeight = ((_a2 = window.visualViewport) === null || _a2 === void 0 ? void 0 : _a2.height) || 0;
      setTimeout(() => {
        var _a22;
        if (
          (((_a22 = window.visualViewport) === null || _a22 === void 0 ? void 0 : _a22.height) || 0) > startViewHeight + 10 && view.hasFocus
        ) {
          view.contentDOM.blur();
          view.focus();
        }
      }, 100,);
    }
  }
};
var wrappingWhiteSpace = ['pre-wrap', 'normal', 'pre-line', 'break-spaces',];
var HeightOracle = class {
  heightForGap(from, to,) {
    let lines = this.doc.lineAt(to,).number - this.doc.lineAt(from,).number + 1;
    if (this.lineWrapping) lines += Math.max(0, Math.ceil((to - from - lines * this.lineLength * 0.5) / this.lineLength,),);
    return this.lineHeight * lines;
  }
  heightForLine(length,) {
    if (!this.lineWrapping) return this.lineHeight;
    let lines = 1 + Math.max(0, Math.ceil((length - this.lineLength) / (this.lineLength - 5),),);
    return lines * this.lineHeight;
  }
  setDoc(doc2,) {
    this.doc = doc2;
    return this;
  }
  mustRefreshForWrapping(whiteSpace,) {
    return wrappingWhiteSpace.indexOf(whiteSpace,) > -1 != this.lineWrapping;
  }
  mustRefreshForHeights(lineHeights,) {
    let newHeight = false;
    for (let i2 = 0; i2 < lineHeights.length; i2++) {
      let h = lineHeights[i2];
      if (h < 0) {
        i2++;
      } else if (!this.heightSamples[Math.floor(h * 10,)]) {
        newHeight = true;
        this.heightSamples[Math.floor(h * 10,)] = true;
      }
    }
    return newHeight;
  }
  refresh(whiteSpace, lineHeight, charWidth, textHeight, lineLength, knownHeights,) {
    let lineWrapping = wrappingWhiteSpace.indexOf(whiteSpace,) > -1;
    let changed = Math.round(lineHeight,) != Math.round(this.lineHeight,) || this.lineWrapping != lineWrapping;
    this.lineWrapping = lineWrapping;
    this.lineHeight = lineHeight;
    this.charWidth = charWidth;
    this.textHeight = textHeight;
    this.lineLength = lineLength;
    if (changed) {
      this.heightSamples = {};
      for (let i2 = 0; i2 < knownHeights.length; i2++) {
        let h = knownHeights[i2];
        if (h < 0) i2++;
        else this.heightSamples[Math.floor(h * 10,)] = true;
      }
    }
    return changed;
  }
  constructor(lineWrapping,) {
    this.lineWrapping = lineWrapping;
    this.doc = Text.empty;
    this.heightSamples = {};
    this.lineHeight = 14;
    this.charWidth = 7;
    this.textHeight = 14;
    this.lineLength = 30;
    this.heightChanged = false;
  }
};
var MeasuredHeights = class {
  get more() {
    return this.index < this.heights.length;
  }
  constructor(from, heights,) {
    this.from = from;
    this.heights = heights;
    this.index = 0;
  }
};
var BlockInfo = class {
  /**
  The type of element this is. When querying lines, this may be
  an array of all the blocks that make up the line.
  */
  get type() {
    return typeof this._content == 'number' ? BlockType.Text : Array.isArray(this._content,) ? this._content : this._content.type;
  }
  /**
  The end of the element as a document position.
  */
  get to() {
    return this.from + this.length;
  }
  /**
  The bottom position of the element.
  */
  get bottom() {
    return this.top + this.height;
  }
  /**
  If this is a widget block, this will return the widget
  associated with it.
  */
  get widget() {
    return this._content instanceof PointDecoration ? this._content.widget : null;
  }
  /**
  If this is a textblock, this holds the number of line breaks
  that appear in widgets inside the block.
  */
  get widgetLineBreaks() {
    return typeof this._content == 'number' ? this._content : 0;
  }
  /**
  @internal
  */
  join(other,) {
    let content2 = (Array.isArray(this._content,) ? this._content : [this,]).concat(
      Array.isArray(other._content,) ? other._content : [other,],
    );
    return new BlockInfo(this.from, this.length + other.length, this.top, this.height + other.height, content2,);
  }
  /**
  @internal
  */
  constructor(from, length, top22, height, _content,) {
    this.from = from;
    this.length = length;
    this.top = top22;
    this.height = height;
    this._content = _content;
  }
};
var QueryType = /* @__PURE__ */ function (QueryType2,) {
  QueryType2[QueryType2['ByPos'] = 0] = 'ByPos';
  QueryType2[QueryType2['ByHeight'] = 1] = 'ByHeight';
  QueryType2[QueryType2['ByPosNoHeight'] = 2] = 'ByPosNoHeight';
  return QueryType2;
}(QueryType || (QueryType = {}),);
var Epsilon = 1e-3;
var HeightMap = class {
  get outdated() {
    return (this.flags & 2) > 0;
  }
  set outdated(value,) {
    this.flags = (value ? 2 : 0) | this.flags & ~2;
  }
  setHeight(oracle, height,) {
    if (this.height != height) {
      if (Math.abs(this.height - height,) > Epsilon) oracle.heightChanged = true;
      this.height = height;
    }
  }
  // Base case is to replace a leaf node, which simply builds a tree
  // from the new nodes and returns that (HeightMapBranch and
  // HeightMapGap override this to actually use from/to)
  replace(_from, _to, nodes,) {
    return HeightMap.of(nodes,);
  }
  // Again, these are base cases, and are overridden for branch and gap nodes.
  decomposeLeft(_to, result,) {
    result.push(this,);
  }
  decomposeRight(_from, result,) {
    result.push(this,);
  }
  applyChanges(decorations2, oldDoc, oracle, changes,) {
    let me = this, doc2 = oracle.doc;
    for (let i2 = changes.length - 1; i2 >= 0; i2--) {
      let { fromA, toA, fromB, toB, } = changes[i2];
      let start = me.lineAt(fromA, QueryType.ByPosNoHeight, oracle.setDoc(oldDoc,), 0, 0,);
      let end = start.to >= toA ? start : me.lineAt(toA, QueryType.ByPosNoHeight, oracle, 0, 0,);
      toB += end.to - toA;
      toA = end.to;
      while (i2 > 0 && start.from <= changes[i2 - 1].toA) {
        fromA = changes[i2 - 1].fromA;
        fromB = changes[i2 - 1].fromB;
        i2--;
        if (fromA < start.from) start = me.lineAt(fromA, QueryType.ByPosNoHeight, oracle, 0, 0,);
      }
      fromB += start.from - fromA;
      fromA = start.from;
      let nodes = NodeBuilder.build(oracle.setDoc(doc2,), decorations2, fromB, toB,);
      me = me.replace(fromA, toA, nodes,);
    }
    return me.updateHeight(oracle, 0,);
  }
  static empty() {
    return new HeightMapText(0, 0,);
  }
  // nodes uses null values to indicate the position of line breaks.
  // There are never line breaks at the start or end of the array, or
  // two line breaks next to each other, and the array isn't allowed
  // to be empty (same restrictions as return value from the builder).
  static of(nodes,) {
    if (nodes.length == 1) return nodes[0];
    let i2 = 0, j = nodes.length, before = 0, after = 0;
    for (;;) {
      if (i2 == j) {
        if (before > after * 2) {
          let split = nodes[i2 - 1];
          if (split.break) nodes.splice(--i2, 1, split.left, null, split.right,);
          else nodes.splice(--i2, 1, split.left, split.right,);
          j += 1 + split.break;
          before -= split.size;
        } else if (after > before * 2) {
          let split1 = nodes[j];
          if (split1.break) nodes.splice(j, 1, split1.left, null, split1.right,);
          else nodes.splice(j, 1, split1.left, split1.right,);
          j += 2 + split1.break;
          after -= split1.size;
        } else {
          break;
        }
      } else if (before < after) {
        let next = nodes[i2++];
        if (next) before += next.size;
      } else {
        let next1 = nodes[--j];
        if (next1) after += next1.size;
      }
    }
    let brk = 0;
    if (nodes[i2 - 1] == null) {
      brk = 1;
      i2--;
    } else if (nodes[i2] == null) {
      brk = 1;
      j++;
    }
    return new HeightMapBranch(HeightMap.of(nodes.slice(0, i2,),), brk, HeightMap.of(nodes.slice(j,),),);
  }
  constructor(length, height, flags = 2,) {
    this.length = length;
    this.height = height;
    this.flags = flags;
  }
};
HeightMap.prototype.size = 1;
var HeightMapBlock = class extends HeightMap {
  blockAt(_height, _oracle, top22, offset,) {
    return new BlockInfo(offset, this.length, top22, this.height, this.deco || 0,);
  }
  lineAt(_value, _type, oracle, top22, offset,) {
    return this.blockAt(0, oracle, top22, offset,);
  }
  forEachLine(from, to, oracle, top22, offset, f,) {
    if (from <= offset + this.length && to >= offset) f(this.blockAt(0, oracle, top22, offset,),);
  }
  updateHeight(oracle, offset = 0, _force = false, measured,) {
    if (measured && measured.from <= offset && measured.more) this.setHeight(oracle, measured.heights[measured.index++],);
    this.outdated = false;
    return this;
  }
  toString() {
    return `block(${this.length})`;
  }
  constructor(length, height, deco,) {
    super(length, height,);
    this.deco = deco;
  }
};
var HeightMapText = class extends HeightMapBlock {
  blockAt(_height, _oracle, top22, offset,) {
    return new BlockInfo(offset, this.length, top22, this.height, this.breaks,);
  }
  replace(_from, _to, nodes,) {
    let node = nodes[0];
    if (
      nodes.length == 1 && (node instanceof HeightMapText || node instanceof HeightMapGap && node.flags & 4) &&
      Math.abs(this.length - node.length,) < 10
    ) {
      if (node instanceof HeightMapGap) node = new HeightMapText(node.length, this.height,);
      else node.height = this.height;
      if (!this.outdated) node.outdated = false;
      return node;
    } else {
      return HeightMap.of(nodes,);
    }
  }
  updateHeight(oracle, offset = 0, force = false, measured,) {
    if (measured && measured.from <= offset && measured.more) this.setHeight(oracle, measured.heights[measured.index++],);
    else if (force || this.outdated) {
      this.setHeight(
        oracle,
        Math.max(this.widgetHeight, oracle.heightForLine(this.length - this.collapsed,),) + this.breaks * oracle.lineHeight,
      );
    }
    this.outdated = false;
    return this;
  }
  toString() {
    return `line(${this.length}${this.collapsed ? -this.collapsed : ''}${this.widgetHeight ? ':' + this.widgetHeight : ''})`;
  }
  constructor(length, height,) {
    super(length, height, null,);
    this.collapsed = 0;
    this.widgetHeight = 0;
    this.breaks = 0;
  }
};
var HeightMapGap = class extends HeightMap {
  heightMetrics(oracle, offset,) {
    let firstLine = oracle.doc.lineAt(offset,).number, lastLine = oracle.doc.lineAt(offset + this.length,).number;
    let lines = lastLine - firstLine + 1;
    let perLine, perChar = 0;
    if (oracle.lineWrapping) {
      let totalPerLine = Math.min(this.height, oracle.lineHeight * lines,);
      perLine = totalPerLine / lines;
      if (this.length > lines + 1) perChar = (this.height - totalPerLine) / (this.length - lines - 1);
    } else {
      perLine = this.height / lines;
    }
    return { firstLine, lastLine, perLine, perChar, };
  }
  blockAt(height, oracle, top22, offset,) {
    let { firstLine, lastLine, perLine, perChar, } = this.heightMetrics(oracle, offset,);
    if (oracle.lineWrapping) {
      let guess = offset + Math.round(Math.max(0, Math.min(1, (height - top22) / this.height,),) * this.length,);
      let line = oracle.doc.lineAt(guess,), lineHeight = perLine + line.length * perChar;
      let lineTop = Math.max(top22, height - lineHeight / 2,);
      return new BlockInfo(line.from, line.length, lineTop, lineHeight, 0,);
    } else {
      let line1 = Math.max(0, Math.min(lastLine - firstLine, Math.floor((height - top22) / perLine,),),);
      let { from, length, } = oracle.doc.line(firstLine + line1,);
      return new BlockInfo(from, length, top22 + perLine * line1, perLine, 0,);
    }
  }
  lineAt(value, type, oracle, top22, offset,) {
    if (type == QueryType.ByHeight) return this.blockAt(value, oracle, top22, offset,);
    if (type == QueryType.ByPosNoHeight) {
      let { from, to, } = oracle.doc.lineAt(value,);
      return new BlockInfo(from, to - from, 0, 0, 0,);
    }
    let { firstLine, perLine, perChar, } = this.heightMetrics(oracle, offset,);
    let line = oracle.doc.lineAt(value,), lineHeight = perLine + line.length * perChar;
    let linesAbove = line.number - firstLine;
    let lineTop = top22 + perLine * linesAbove + perChar * (line.from - offset - linesAbove);
    return new BlockInfo(line.from, line.length, Math.max(top22, Math.min(lineTop, top22 + this.height - lineHeight,),), lineHeight, 0,);
  }
  forEachLine(from, to, oracle, top22, offset, f,) {
    from = Math.max(from, offset,);
    to = Math.min(to, offset + this.length,);
    let { firstLine, perLine, perChar, } = this.heightMetrics(oracle, offset,);
    for (let pos = from, lineTop = top22; pos <= to;) {
      let line = oracle.doc.lineAt(pos,);
      if (pos == from) {
        let linesAbove = line.number - firstLine;
        lineTop += perLine * linesAbove + perChar * (from - offset - linesAbove);
      }
      let lineHeight = perLine + perChar * line.length;
      f(new BlockInfo(line.from, line.length, lineTop, lineHeight, 0,),);
      lineTop += lineHeight;
      pos = line.to + 1;
    }
  }
  replace(from, to, nodes,) {
    let after = this.length - to;
    if (after > 0) {
      let last = nodes[nodes.length - 1];
      if (last instanceof HeightMapGap) nodes[nodes.length - 1] = new HeightMapGap(last.length + after,);
      else nodes.push(null, new HeightMapGap(after - 1,),);
    }
    if (from > 0) {
      let first = nodes[0];
      if (first instanceof HeightMapGap) nodes[0] = new HeightMapGap(from + first.length,);
      else nodes.unshift(new HeightMapGap(from - 1,), null,);
    }
    return HeightMap.of(nodes,);
  }
  decomposeLeft(to, result,) {
    result.push(new HeightMapGap(to - 1,), null,);
  }
  decomposeRight(from, result,) {
    result.push(null, new HeightMapGap(this.length - from - 1,),);
  }
  updateHeight(oracle, offset = 0, force = false, measured,) {
    let end = offset + this.length;
    if (measured && measured.from <= offset + this.length && measured.more) {
      let nodes = [], pos = Math.max(offset, measured.from,), singleHeight = -1;
      if (measured.from > offset) nodes.push(new HeightMapGap(measured.from - offset - 1,).updateHeight(oracle, offset,),);
      while (pos <= end && measured.more) {
        let len = oracle.doc.lineAt(pos,).length;
        if (nodes.length) nodes.push(null,);
        let height = measured.heights[measured.index++];
        if (singleHeight == -1) singleHeight = height;
        else if (Math.abs(height - singleHeight,) >= Epsilon) singleHeight = -2;
        let line = new HeightMapText(len, height,);
        line.outdated = false;
        nodes.push(line,);
        pos += len + 1;
      }
      if (pos <= end) nodes.push(null, new HeightMapGap(end - pos,).updateHeight(oracle, pos,),);
      let result = HeightMap.of(nodes,);
      if (
        singleHeight < 0 || Math.abs(result.height - this.height,) >= Epsilon ||
        Math.abs(singleHeight - this.heightMetrics(oracle, offset,).perLine,) >= Epsilon
      ) oracle.heightChanged = true;
      return result;
    } else if (force || this.outdated) {
      this.setHeight(oracle, oracle.heightForGap(offset, offset + this.length,),);
      this.outdated = false;
    }
    return this;
  }
  toString() {
    return `gap(${this.length})`;
  }
  constructor(length,) {
    super(length, 0,);
  }
};
var HeightMapBranch = class extends HeightMap {
  get break() {
    return this.flags & 1;
  }
  blockAt(height, oracle, top22, offset,) {
    let mid = top22 + this.left.height;
    return height < mid
      ? this.left.blockAt(height, oracle, top22, offset,)
      : this.right.blockAt(height, oracle, mid, offset + this.left.length + this.break,);
  }
  lineAt(value, type, oracle, top22, offset,) {
    let rightTop = top22 + this.left.height, rightOffset = offset + this.left.length + this.break;
    let left = type == QueryType.ByHeight ? value < rightTop : value < rightOffset;
    let base2 = left
      ? this.left.lineAt(value, type, oracle, top22, offset,)
      : this.right.lineAt(value, type, oracle, rightTop, rightOffset,);
    if (this.break || (left ? base2.to < rightOffset : base2.from > rightOffset)) return base2;
    let subQuery = type == QueryType.ByPosNoHeight ? QueryType.ByPosNoHeight : QueryType.ByPos;
    if (left) return base2.join(this.right.lineAt(rightOffset, subQuery, oracle, rightTop, rightOffset,),);
    else return this.left.lineAt(rightOffset, subQuery, oracle, top22, offset,).join(base2,);
  }
  forEachLine(from, to, oracle, top22, offset, f,) {
    let rightTop = top22 + this.left.height, rightOffset = offset + this.left.length + this.break;
    if (this.break) {
      if (from < rightOffset) this.left.forEachLine(from, to, oracle, top22, offset, f,);
      if (to >= rightOffset) this.right.forEachLine(from, to, oracle, rightTop, rightOffset, f,);
    } else {
      let mid = this.lineAt(rightOffset, QueryType.ByPos, oracle, top22, offset,);
      if (from < mid.from) this.left.forEachLine(from, mid.from - 1, oracle, top22, offset, f,);
      if (mid.to >= from && mid.from <= to) f(mid,);
      if (to > mid.to) this.right.forEachLine(mid.to + 1, to, oracle, rightTop, rightOffset, f,);
    }
  }
  replace(from, to, nodes,) {
    let rightStart = this.left.length + this.break;
    if (to < rightStart) return this.balanced(this.left.replace(from, to, nodes,), this.right,);
    if (from > this.left.length) return this.balanced(this.left, this.right.replace(from - rightStart, to - rightStart, nodes,),);
    let result = [];
    if (from > 0) this.decomposeLeft(from, result,);
    let left = result.length;
    for (let node of nodes) result.push(node,);
    if (from > 0) mergeGaps(result, left - 1,);
    if (to < this.length) {
      let right = result.length;
      this.decomposeRight(to, result,);
      mergeGaps(result, right,);
    }
    return HeightMap.of(result,);
  }
  decomposeLeft(to, result,) {
    let left = this.left.length;
    if (to <= left) return this.left.decomposeLeft(to, result,);
    result.push(this.left,);
    if (this.break) {
      left++;
      if (to >= left) result.push(null,);
    }
    if (to > left) this.right.decomposeLeft(to - left, result,);
  }
  decomposeRight(from, result,) {
    let left = this.left.length, right = left + this.break;
    if (from >= right) return this.right.decomposeRight(from - right, result,);
    if (from < left) this.left.decomposeRight(from, result,);
    if (this.break && from < right) result.push(null,);
    result.push(this.right,);
  }
  balanced(left, right,) {
    if (left.size > 2 * right.size || right.size > 2 * left.size) return HeightMap.of(this.break ? [left, null, right,] : [left, right,],);
    this.left = left;
    this.right = right;
    this.height = left.height + right.height;
    this.outdated = left.outdated || right.outdated;
    this.size = left.size + right.size;
    this.length = left.length + this.break + right.length;
    return this;
  }
  updateHeight(oracle, offset = 0, force = false, measured,) {
    let { left, right, } = this, rightStart = offset + left.length + this.break, rebalance = null;
    if (measured && measured.from <= offset + left.length && measured.more) {
      rebalance = left = left.updateHeight(oracle, offset, force, measured,);
    } else left.updateHeight(oracle, offset, force,);
    if (measured && measured.from <= rightStart + right.length && measured.more) {
      rebalance = right = right.updateHeight(oracle, rightStart, force, measured,);
    } else right.updateHeight(oracle, rightStart, force,);
    if (rebalance) return this.balanced(left, right,);
    this.height = this.left.height + this.right.height;
    this.outdated = false;
    return this;
  }
  toString() {
    return this.left + (this.break ? ' ' : '-') + this.right;
  }
  constructor(left, brk, right,) {
    super(left.length + brk + right.length, left.height + right.height, brk | (left.outdated || right.outdated ? 2 : 0),);
    this.left = left;
    this.right = right;
    this.size = left.size + right.size;
  }
};
function mergeGaps(nodes, around,) {
  let before, after;
  if (
    nodes[around] == null && (before = nodes[around - 1]) instanceof HeightMapGap && (after = nodes[around + 1]) instanceof HeightMapGap
  ) nodes.splice(around - 1, 3, new HeightMapGap(before.length + 1 + after.length,),);
}
var relevantWidgetHeight = 5;
var NodeBuilder = class {
  get isCovered() {
    return this.covering && this.nodes[this.nodes.length - 1] == this.covering;
  }
  span(_from, to,) {
    if (this.lineStart > -1) {
      let end = Math.min(to, this.lineEnd,), last = this.nodes[this.nodes.length - 1];
      if (last instanceof HeightMapText) last.length += end - this.pos;
      else if (end > this.pos || !this.isCovered) this.nodes.push(new HeightMapText(end - this.pos, -1,),);
      this.writtenTo = end;
      if (to > end) {
        this.nodes.push(null,);
        this.writtenTo++;
        this.lineStart = -1;
      }
    }
    this.pos = to;
  }
  point(from, to, deco,) {
    if (from < to || deco.heightRelevant) {
      let height = deco.widget ? deco.widget.estimatedHeight : 0;
      let breaks = deco.widget ? deco.widget.lineBreaks : 0;
      if (height < 0) height = this.oracle.lineHeight;
      let len = to - from;
      if (deco.block) {
        this.addBlock(new HeightMapBlock(len, height, deco,),);
      } else if (len || breaks || height >= relevantWidgetHeight) {
        this.addLineDeco(height, breaks, len,);
      }
    } else if (to > from) {
      this.span(from, to,);
    }
    if (this.lineEnd > -1 && this.lineEnd < this.pos) this.lineEnd = this.oracle.doc.lineAt(this.pos,).to;
  }
  enterLine() {
    if (this.lineStart > -1) return;
    let { from, to, } = this.oracle.doc.lineAt(this.pos,);
    this.lineStart = from;
    this.lineEnd = to;
    if (this.writtenTo < from) {
      if (this.writtenTo < from - 1 || this.nodes[this.nodes.length - 1] == null) {
        this.nodes.push(this.blankContent(this.writtenTo, from - 1,),);
      }
      this.nodes.push(null,);
    }
    if (this.pos > from) this.nodes.push(new HeightMapText(this.pos - from, -1,),);
    this.writtenTo = this.pos;
  }
  blankContent(from, to,) {
    let gap = new HeightMapGap(to - from,);
    if (this.oracle.doc.lineAt(from,).to == to) gap.flags |= 4;
    return gap;
  }
  ensureLine() {
    this.enterLine();
    let last = this.nodes.length ? this.nodes[this.nodes.length - 1] : null;
    if (last instanceof HeightMapText) return last;
    let line = new HeightMapText(0, -1,);
    this.nodes.push(line,);
    return line;
  }
  addBlock(block,) {
    var _a2;
    this.enterLine();
    let type = (_a2 = block.deco) === null || _a2 === void 0 ? void 0 : _a2.type;
    if (type == BlockType.WidgetAfter && !this.isCovered) this.ensureLine();
    this.nodes.push(block,);
    this.writtenTo = this.pos = this.pos + block.length;
    if (type != BlockType.WidgetBefore) this.covering = block;
  }
  addLineDeco(height, breaks, length,) {
    let line = this.ensureLine();
    line.length += length;
    line.collapsed += length;
    line.widgetHeight = Math.max(line.widgetHeight, height,);
    line.breaks += breaks;
    this.writtenTo = this.pos = this.pos + length;
  }
  finish(from,) {
    let last = this.nodes.length == 0 ? null : this.nodes[this.nodes.length - 1];
    if (this.lineStart > -1 && !(last instanceof HeightMapText) && !this.isCovered) this.nodes.push(new HeightMapText(0, -1,),);
    else if (this.writtenTo < this.pos || last == null) this.nodes.push(this.blankContent(this.writtenTo, this.pos,),);
    let pos = from;
    for (let node of this.nodes) {
      if (node instanceof HeightMapText) node.updateHeight(this.oracle, pos,);
      pos += node ? node.length : 1;
    }
    return this.nodes;
  }
  // Always called with a region that on both sides either stretches
  // to a line break or the end of the document.
  // The returned array uses null to indicate line breaks, but never
  // starts or ends in a line break, or has multiple line breaks next
  // to each other.
  static build(oracle, decorations2, from, to,) {
    let builder = new NodeBuilder(from, oracle,);
    RangeSet.spans(decorations2, from, to, builder, 0,);
    return builder.finish(from,);
  }
  constructor(pos, oracle,) {
    this.pos = pos;
    this.oracle = oracle;
    this.nodes = [];
    this.lineStart = -1;
    this.lineEnd = -1;
    this.covering = null;
    this.writtenTo = pos;
  }
};
function heightRelevantDecoChanges(a, b, diff,) {
  let comp = new DecorationComparator();
  RangeSet.compare(a, b, diff, comp, 0,);
  return comp.changes;
}
var DecorationComparator = class {
  compareRange() {
  }
  comparePoint(from, to, a, b,) {
    if (from < to || a && a.heightRelevant || b && b.heightRelevant) addRange(from, to, this.changes, 5,);
  }
  constructor() {
    this.changes = [];
  }
};
function visiblePixelRange(dom, paddingTop,) {
  let rect = dom.getBoundingClientRect();
  let doc2 = dom.ownerDocument, win = doc2.defaultView || window;
  let left = Math.max(0, rect.left,), right = Math.min(win.innerWidth, rect.right,);
  let top22 = Math.max(0, rect.top,), bottom = Math.min(win.innerHeight, rect.bottom,);
  for (let parent = dom.parentNode; parent && parent != doc2.body;) {
    if (parent.nodeType == 1) {
      let elt = parent;
      let style = window.getComputedStyle(elt,);
      if ((elt.scrollHeight > elt.clientHeight || elt.scrollWidth > elt.clientWidth) && style.overflow != 'visible') {
        let parentRect = elt.getBoundingClientRect();
        left = Math.max(left, parentRect.left,);
        right = Math.min(right, parentRect.right,);
        top22 = Math.max(top22, parentRect.top,);
        bottom = parent == dom.parentNode ? parentRect.bottom : Math.min(bottom, parentRect.bottom,);
      }
      parent = style.position == 'absolute' || style.position == 'fixed' ? elt.offsetParent : elt.parentNode;
    } else if (parent.nodeType == 11) {
      parent = parent.host;
    } else {
      break;
    }
  }
  return {
    left: left - rect.left,
    right: Math.max(left, right,) - rect.left,
    top: top22 - (rect.top + paddingTop),
    bottom: Math.max(top22, bottom,) - (rect.top + paddingTop),
  };
}
function fullPixelRange(dom, paddingTop,) {
  let rect = dom.getBoundingClientRect();
  return { left: 0, right: rect.right - rect.left, top: paddingTop, bottom: rect.bottom - (rect.top + paddingTop), };
}
var LineGap = class {
  static same(a, b,) {
    if (a.length != b.length) return false;
    for (let i2 = 0; i2 < a.length; i2++) {
      let gA = a[i2], gB = b[i2];
      if (gA.from != gB.from || gA.to != gB.to || gA.size != gB.size) return false;
    }
    return true;
  }
  draw(wrapping,) {
    return Decoration.replace({ widget: new LineGapWidget(this.size, wrapping,), },).range(this.from, this.to,);
  }
  constructor(from, to, size,) {
    this.from = from;
    this.to = to;
    this.size = size;
  }
};
var LineGapWidget = class extends WidgetType {
  eq(other,) {
    return other.size == this.size && other.vertical == this.vertical;
  }
  toDOM() {
    let elt = document.createElement('div',);
    if (this.vertical) {
      elt.style.height = this.size + 'px';
    } else {
      elt.style.width = this.size + 'px';
      elt.style.height = '2px';
      elt.style.display = 'inline-block';
    }
    return elt;
  }
  get estimatedHeight() {
    return this.vertical ? this.size : -1;
  }
  constructor(size, vertical,) {
    super();
    this.size = size;
    this.vertical = vertical;
  }
};
var ViewState = class {
  updateForViewport() {
    let viewports = [this.viewport,], { main, } = this.state.selection;
    for (let i2 = 0; i2 <= 1; i2++) {
      let pos = i2 ? main.head : main.anchor;
      if (!viewports.some(({ from, to, },) => pos >= from && pos <= to)) {
        let { from, to, } = this.lineBlockAt(pos,);
        viewports.push(new Viewport(from, to,),);
      }
    }
    this.viewports = viewports.sort((a, b,) => a.from - b.from);
    this.scaler = this.heightMap.height <= 7e6 ? IdScaler : new BigScaler(this.heightOracle, this.heightMap, this.viewports,);
  }
  updateViewportLines() {
    this.viewportLines = [];
    this.heightMap.forEachLine(this.viewport.from, this.viewport.to, this.heightOracle.setDoc(this.state.doc,), 0, 0, (block,) => {
      this.viewportLines.push(this.scaler.scale == 1 ? block : scaleBlock(block, this.scaler,),);
    },);
  }
  update(update, scrollTarget = null,) {
    this.state = update.state;
    let prevDeco = this.stateDeco;
    this.stateDeco = this.state.facet(decorations,).filter((d,) => typeof d != 'function');
    let contentChanges = update.changedRanges;
    let heightChanges = ChangedRange.extendWithRanges(
      contentChanges,
      heightRelevantDecoChanges(prevDeco, this.stateDeco, update ? update.changes : ChangeSet.empty(this.state.doc.length,),),
    );
    let prevHeight = this.heightMap.height;
    let scrollAnchor = this.scrolledToBottom ? null : this.lineBlockAtHeight(this.scrollTop,);
    this.heightMap = this.heightMap.applyChanges(
      this.stateDeco,
      update.startState.doc,
      this.heightOracle.setDoc(this.state.doc,),
      heightChanges,
    );
    if (this.heightMap.height != prevHeight) update.flags |= 2;
    if (scrollAnchor) {
      this.scrollAnchorPos = update.changes.mapPos(scrollAnchor.from, -1,);
      this.scrollAnchorHeight = scrollAnchor.top;
    } else {
      this.scrollAnchorPos = -1;
      this.scrollAnchorHeight = this.heightMap.height;
    }
    let viewport = heightChanges.length ? this.mapViewport(this.viewport, update.changes,) : this.viewport;
    if (
      scrollTarget && (scrollTarget.range.head < viewport.from || scrollTarget.range.head > viewport.to) ||
      !this.viewportIsAppropriate(viewport,)
    ) viewport = this.getViewport(0, scrollTarget,);
    let updateLines = !update.changes.empty || update.flags & 2 || viewport.from != this.viewport.from || viewport.to != this.viewport.to;
    this.viewport = viewport;
    this.updateForViewport();
    if (updateLines) this.updateViewportLines();
    if (this.lineGaps.length || this.viewport.to - this.viewport.from > 2e3 << 1) {
      this.updateLineGaps(this.ensureLineGaps(this.mapLineGaps(this.lineGaps, update.changes,),),);
    }
    update.flags |= this.computeVisibleRanges();
    if (scrollTarget) this.scrollTarget = scrollTarget;
    if (
      !this.mustEnforceCursorAssoc && update.selectionSet && update.view.lineWrapping && update.state.selection.main.empty &&
      update.state.selection.main.assoc && !update.state.facet(nativeSelectionHidden,)
    ) this.mustEnforceCursorAssoc = true;
  }
  measure(view,) {
    let dom = view.contentDOM, style = window.getComputedStyle(dom,);
    let oracle = this.heightOracle;
    let whiteSpace = style.whiteSpace;
    this.defaultTextDirection = style.direction == 'rtl' ? Direction.RTL : Direction.LTR;
    let refresh = this.heightOracle.mustRefreshForWrapping(whiteSpace,);
    let domRect = dom.getBoundingClientRect();
    let measureContent = refresh || this.mustMeasureContent || this.contentDOMHeight != domRect.height;
    this.contentDOMHeight = domRect.height;
    this.mustMeasureContent = false;
    let result = 0, bias = 0;
    let paddingTop = parseInt(style.paddingTop,) || 0, paddingBottom = parseInt(style.paddingBottom,) || 0;
    if (this.paddingTop != paddingTop || this.paddingBottom != paddingBottom) {
      this.paddingTop = paddingTop;
      this.paddingBottom = paddingBottom;
      result |= 8 | 2;
    }
    if (this.editorWidth != view.scrollDOM.clientWidth) {
      if (oracle.lineWrapping) measureContent = true;
      this.editorWidth = view.scrollDOM.clientWidth;
      result |= 8;
    }
    if (this.scrollTop != view.scrollDOM.scrollTop) {
      this.scrollAnchorHeight = -1;
      this.scrollTop = view.scrollDOM.scrollTop;
    }
    this.scrolledToBottom = this.scrollTop > view.scrollDOM.scrollHeight - view.scrollDOM.clientHeight - 4;
    let pixelViewport = (this.printing ? fullPixelRange : visiblePixelRange)(dom, this.paddingTop,);
    let dTop = pixelViewport.top - this.pixelViewport.top, dBottom = pixelViewport.bottom - this.pixelViewport.bottom;
    this.pixelViewport = pixelViewport;
    let inView = this.pixelViewport.bottom > this.pixelViewport.top && this.pixelViewport.right > this.pixelViewport.left;
    if (inView != this.inView) {
      this.inView = inView;
      if (inView) measureContent = true;
    }
    if (!this.inView && !this.scrollTarget) return 0;
    let contentWidth = domRect.width;
    if (this.contentDOMWidth != contentWidth || this.editorHeight != view.scrollDOM.clientHeight) {
      this.contentDOMWidth = domRect.width;
      this.editorHeight = view.scrollDOM.clientHeight;
      result |= 8;
    }
    if (measureContent) {
      let lineHeights = view.docView.measureVisibleLineHeights(this.viewport,);
      if (oracle.mustRefreshForHeights(lineHeights,)) refresh = true;
      if (refresh || oracle.lineWrapping && Math.abs(contentWidth - this.contentDOMWidth,) > oracle.charWidth) {
        let { lineHeight, charWidth, textHeight, } = view.docView.measureTextSize();
        refresh = lineHeight > 0 && oracle.refresh(whiteSpace, lineHeight, charWidth, textHeight, contentWidth / charWidth, lineHeights,);
        if (refresh) {
          view.docView.minWidth = 0;
          result |= 8;
        }
      }
      if (dTop > 0 && dBottom > 0) bias = Math.max(dTop, dBottom,);
      else if (dTop < 0 && dBottom < 0) bias = Math.min(dTop, dBottom,);
      oracle.heightChanged = false;
      for (let vp of this.viewports) {
        let heights = vp.from == this.viewport.from ? lineHeights : view.docView.measureVisibleLineHeights(vp,);
        this.heightMap = (refresh
          ? HeightMap.empty().applyChanges(this.stateDeco, Text.empty, this.heightOracle, [
            new ChangedRange(0, 0, 0, view.state.doc.length,),
          ],)
          : this.heightMap).updateHeight(oracle, 0, refresh, new MeasuredHeights(vp.from, heights,),);
      }
      if (oracle.heightChanged) result |= 2;
    }
    let viewportChange = !this.viewportIsAppropriate(this.viewport, bias,) ||
      this.scrollTarget && (this.scrollTarget.range.head < this.viewport.from || this.scrollTarget.range.head > this.viewport.to);
    if (viewportChange) this.viewport = this.getViewport(bias, this.scrollTarget,);
    this.updateForViewport();
    if (result & 2 || viewportChange) this.updateViewportLines();
    if (this.lineGaps.length || this.viewport.to - this.viewport.from > 2e3 << 1) {
      this.updateLineGaps(this.ensureLineGaps(refresh ? [] : this.lineGaps, view,),);
    }
    result |= this.computeVisibleRanges();
    if (this.mustEnforceCursorAssoc) {
      this.mustEnforceCursorAssoc = false;
      view.docView.enforceCursorAssoc();
    }
    return result;
  }
  get visibleTop() {
    return this.scaler.fromDOM(this.pixelViewport.top,);
  }
  get visibleBottom() {
    return this.scaler.fromDOM(this.pixelViewport.bottom,);
  }
  getViewport(bias, scrollTarget,) {
    let marginTop = 0.5 - Math.max(-0.5, Math.min(0.5, bias / 1e3 / 2,),);
    let map = this.heightMap, oracle = this.heightOracle;
    let { visibleTop, visibleBottom, } = this;
    let viewport = new Viewport(
      map.lineAt(visibleTop - marginTop * 1e3, QueryType.ByHeight, oracle, 0, 0,).from,
      map.lineAt(visibleBottom + (1 - marginTop) * 1e3, QueryType.ByHeight, oracle, 0, 0,).to,
    );
    if (scrollTarget) {
      let { head, } = scrollTarget.range;
      if (head < viewport.from || head > viewport.to) {
        let viewHeight = Math.min(this.editorHeight, this.pixelViewport.bottom - this.pixelViewport.top,);
        let block = map.lineAt(head, QueryType.ByPos, oracle, 0, 0,), topPos;
        if (scrollTarget.y == 'center') topPos = (block.top + block.bottom) / 2 - viewHeight / 2;
        else if (scrollTarget.y == 'start' || scrollTarget.y == 'nearest' && head < viewport.from) topPos = block.top;
        else topPos = block.bottom - viewHeight;
        viewport = new Viewport(
          map.lineAt(topPos - 1e3 / 2, QueryType.ByHeight, oracle, 0, 0,).from,
          map.lineAt(topPos + viewHeight + 1e3 / 2, QueryType.ByHeight, oracle, 0, 0,).to,
        );
      }
    }
    return viewport;
  }
  mapViewport(viewport, changes,) {
    let from = changes.mapPos(viewport.from, -1,), to = changes.mapPos(viewport.to, 1,);
    return new Viewport(
      this.heightMap.lineAt(from, QueryType.ByPos, this.heightOracle, 0, 0,).from,
      this.heightMap.lineAt(to, QueryType.ByPos, this.heightOracle, 0, 0,).to,
    );
  }
  // Checks if a given viewport covers the visible part of the
  // document and not too much beyond that.
  viewportIsAppropriate({ from, to, }, bias = 0,) {
    if (!this.inView) return true;
    let { top: top22, } = this.heightMap.lineAt(from, QueryType.ByPos, this.heightOracle, 0, 0,);
    let { bottom, } = this.heightMap.lineAt(to, QueryType.ByPos, this.heightOracle, 0, 0,);
    let { visibleTop, visibleBottom, } = this;
    return (from == 0 || top22 <= visibleTop - Math.max(10, Math.min(-bias, 250,),)) &&
      (to == this.state.doc.length || bottom >= visibleBottom + Math.max(10, Math.min(bias, 250,),)) && top22 > visibleTop - 2 * 1e3 &&
      bottom < visibleBottom + 2 * 1e3;
  }
  mapLineGaps(gaps, changes,) {
    if (!gaps.length || changes.empty) return gaps;
    let mapped = [];
    for (let gap of gaps) {
      if (!changes.touchesRange(gap.from, gap.to,)) {
        mapped.push(new LineGap(changes.mapPos(gap.from,), changes.mapPos(gap.to,), gap.size,),);
      }
    }
    return mapped;
  }
  // Computes positions in the viewport where the start or end of a
  // line should be hidden, trying to reuse existing line gaps when
  // appropriate to avoid unneccesary redraws.
  // Uses crude character-counting for the positioning and sizing,
  // since actual DOM coordinates aren't always available and
  // predictable. Relies on generous margins (see LG.Margin) to hide
  // the artifacts this might produce from the user.
  ensureLineGaps(current, mayMeasure,) {
    let wrapping = this.heightOracle.lineWrapping;
    let margin = wrapping ? 1e4 : 2e3, halfMargin = margin >> 1, doubleMargin = margin << 1;
    if (this.defaultTextDirection != Direction.LTR && !wrapping) return [];
    let gaps = [];
    let addGap = (from, to, line, structure,) => {
      if (to - from < halfMargin) return;
      let sel = this.state.selection.main, avoid = [sel.from,];
      if (!sel.empty) avoid.push(sel.to,);
      for (let pos of avoid) {
        if (pos > from && pos < to) {
          addGap(from, pos - 10, line, structure,);
          addGap(pos + 10, to, line, structure,);
          return;
        }
      }
      let gap = find(
        current,
        (gap2,) =>
          gap2.from >= line.from && gap2.to <= line.to && Math.abs(gap2.from - from,) < halfMargin &&
          Math.abs(gap2.to - to,) < halfMargin && !avoid.some((pos,) => gap2.from < pos && gap2.to > pos),
      );
      if (!gap) {
        if (to < line.to && mayMeasure && wrapping && mayMeasure.visibleRanges.some((r,) => r.from <= to && r.to >= to)) {
          let lineStart = mayMeasure.moveToLineBoundary(EditorSelection.cursor(to,), false, true,).head;
          if (lineStart > from) to = lineStart;
        }
        gap = new LineGap(from, to, this.gapSize(line, from, to, structure,),);
      }
      gaps.push(gap,);
    };
    for (let line of this.viewportLines) {
      if (line.length < doubleMargin) continue;
      let structure = lineStructure(line.from, line.to, this.stateDeco,);
      if (structure.total < doubleMargin) continue;
      let target = this.scrollTarget ? this.scrollTarget.range.head : null;
      let viewFrom, viewTo;
      if (wrapping) {
        let marginHeight = margin / this.heightOracle.lineLength * this.heightOracle.lineHeight;
        let top22, bot;
        if (target != null) {
          let targetFrac = findFraction(structure, target,);
          let spaceFrac = ((this.visibleBottom - this.visibleTop) / 2 + marginHeight) / line.height;
          top22 = targetFrac - spaceFrac;
          bot = targetFrac + spaceFrac;
        } else {
          top22 = (this.visibleTop - line.top - marginHeight) / line.height;
          bot = (this.visibleBottom - line.top + marginHeight) / line.height;
        }
        viewFrom = findPosition(structure, top22,);
        viewTo = findPosition(structure, bot,);
      } else {
        let totalWidth = structure.total * this.heightOracle.charWidth;
        let marginWidth = margin * this.heightOracle.charWidth;
        let left, right;
        if (target != null) {
          let targetFrac1 = findFraction(structure, target,);
          let spaceFrac1 = ((this.pixelViewport.right - this.pixelViewport.left) / 2 + marginWidth) / totalWidth;
          left = targetFrac1 - spaceFrac1;
          right = targetFrac1 + spaceFrac1;
        } else {
          left = (this.pixelViewport.left - marginWidth) / totalWidth;
          right = (this.pixelViewport.right + marginWidth) / totalWidth;
        }
        viewFrom = findPosition(structure, left,);
        viewTo = findPosition(structure, right,);
      }
      if (viewFrom > line.from) addGap(line.from, viewFrom, line, structure,);
      if (viewTo < line.to) addGap(viewTo, line.to, line, structure,);
    }
    return gaps;
  }
  gapSize(line, from, to, structure,) {
    let fraction = findFraction(structure, to,) - findFraction(structure, from,);
    if (this.heightOracle.lineWrapping) {
      return line.height * fraction;
    } else {
      return structure.total * this.heightOracle.charWidth * fraction;
    }
  }
  updateLineGaps(gaps,) {
    if (!LineGap.same(gaps, this.lineGaps,)) {
      this.lineGaps = gaps;
      this.lineGapDeco = Decoration.set(gaps.map((gap,) => gap.draw(this.heightOracle.lineWrapping,)),);
    }
  }
  computeVisibleRanges() {
    let deco = this.stateDeco;
    if (this.lineGaps.length) deco = deco.concat(this.lineGapDeco,);
    let ranges = [];
    RangeSet.spans(deco, this.viewport.from, this.viewport.to, {
      span(from, to,) {
        ranges.push({ from, to, },);
      },
      point() {
      },
    }, 20,);
    let changed = ranges.length != this.visibleRanges.length ||
      this.visibleRanges.some((r, i2,) => r.from != ranges[i2].from || r.to != ranges[i2].to);
    this.visibleRanges = ranges;
    return changed ? 4 : 0;
  }
  lineBlockAt(pos,) {
    return pos >= this.viewport.from && pos <= this.viewport.to && this.viewportLines.find((b,) => b.from <= pos && b.to >= pos) ||
      scaleBlock(this.heightMap.lineAt(pos, QueryType.ByPos, this.heightOracle, 0, 0,), this.scaler,);
  }
  lineBlockAtHeight(height,) {
    return scaleBlock(this.heightMap.lineAt(this.scaler.fromDOM(height,), QueryType.ByHeight, this.heightOracle, 0, 0,), this.scaler,);
  }
  elementAtHeight(height,) {
    return scaleBlock(this.heightMap.blockAt(this.scaler.fromDOM(height,), this.heightOracle, 0, 0,), this.scaler,);
  }
  get docHeight() {
    return this.scaler.toDOM(this.heightMap.height,);
  }
  get contentHeight() {
    return this.docHeight + this.paddingTop + this.paddingBottom;
  }
  constructor(state,) {
    this.state = state;
    this.pixelViewport = { left: 0, right: window.innerWidth, top: 0, bottom: 0, };
    this.inView = true;
    this.paddingTop = 0;
    this.paddingBottom = 0;
    this.contentDOMWidth = 0;
    this.contentDOMHeight = 0;
    this.editorHeight = 0;
    this.editorWidth = 0;
    this.scrollTop = 0;
    this.scrolledToBottom = true;
    this.scrollAnchorPos = 0;
    this.scrollAnchorHeight = -1;
    this.scaler = IdScaler;
    this.scrollTarget = null;
    this.printing = false;
    this.mustMeasureContent = true;
    this.defaultTextDirection = Direction.LTR;
    this.visibleRanges = [];
    this.mustEnforceCursorAssoc = false;
    let guessWrapping = state.facet(contentAttributes,).some((v,) => typeof v != 'function' && v.class == 'cm-lineWrapping');
    this.heightOracle = new HeightOracle(guessWrapping,);
    this.stateDeco = state.facet(decorations,).filter((d,) => typeof d != 'function');
    this.heightMap = HeightMap.empty().applyChanges(this.stateDeco, Text.empty, this.heightOracle.setDoc(state.doc,), [
      new ChangedRange(0, 0, 0, state.doc.length,),
    ],);
    this.viewport = this.getViewport(0, null,);
    this.updateViewportLines();
    this.updateForViewport();
    this.lineGaps = this.ensureLineGaps([],);
    this.lineGapDeco = Decoration.set(this.lineGaps.map((gap,) => gap.draw(false,)),);
    this.computeVisibleRanges();
  }
};
var Viewport = class {
  constructor(from, to,) {
    this.from = from;
    this.to = to;
  }
};
function lineStructure(from, to, stateDeco,) {
  let ranges = [], pos = from, total = 0;
  RangeSet.spans(stateDeco, from, to, {
    span() {
    },
    point(from2, to2,) {
      if (from2 > pos) {
        ranges.push({ from: pos, to: from2, },);
        total += from2 - pos;
      }
      pos = to2;
    },
  }, 20,);
  if (pos < to) {
    ranges.push({ from: pos, to, },);
    total += to - pos;
  }
  return { total, ranges, };
}
function findPosition({ total, ranges, }, ratio,) {
  if (ratio <= 0) return ranges[0].from;
  if (ratio >= 1) return ranges[ranges.length - 1].to;
  let dist = Math.floor(total * ratio,);
  for (let i2 = 0;; i2++) {
    let { from, to, } = ranges[i2], size = to - from;
    if (dist <= size) return from + dist;
    dist -= size;
  }
}
function findFraction(structure, pos,) {
  let counted = 0;
  for (let { from, to, } of structure.ranges) {
    if (pos <= to) {
      counted += pos - from;
      break;
    }
    counted += to - from;
  }
  return counted / structure.total;
}
function find(array, f,) {
  for (let val of array) if (f(val,)) return val;
  return void 0;
}
var IdScaler = {
  toDOM(n,) {
    return n;
  },
  fromDOM(n,) {
    return n;
  },
  scale: 1,
};
var BigScaler = class {
  toDOM(n,) {
    for (let i2 = 0, base2 = 0, domBase = 0;; i2++) {
      let vp = i2 < this.viewports.length ? this.viewports[i2] : null;
      if (!vp || n < vp.top) return domBase + (n - base2) * this.scale;
      if (n <= vp.bottom) return vp.domTop + (n - vp.top);
      base2 = vp.bottom;
      domBase = vp.domBottom;
    }
  }
  fromDOM(n,) {
    for (let i2 = 0, base2 = 0, domBase = 0;; i2++) {
      let vp = i2 < this.viewports.length ? this.viewports[i2] : null;
      if (!vp || n < vp.domTop) return base2 + (n - domBase) / this.scale;
      if (n <= vp.domBottom) return vp.top + (n - vp.domTop);
      base2 = vp.bottom;
      domBase = vp.domBottom;
    }
  }
  constructor(oracle, heightMap, viewports,) {
    let vpHeight = 0, base2 = 0, domBase = 0;
    this.viewports = viewports.map(({ from, to, },) => {
      let top22 = heightMap.lineAt(from, QueryType.ByPos, oracle, 0, 0,).top;
      let bottom = heightMap.lineAt(to, QueryType.ByPos, oracle, 0, 0,).bottom;
      vpHeight += bottom - top22;
      return { from, to, top: top22, bottom, domTop: 0, domBottom: 0, };
    },);
    this.scale = (7e6 - vpHeight) / (heightMap.height - vpHeight);
    for (let obj of this.viewports) {
      obj.domTop = domBase + (obj.top - base2) * this.scale;
      domBase = obj.domBottom = obj.domTop + (obj.bottom - obj.top);
      base2 = obj.bottom;
    }
  }
};
function scaleBlock(block, scaler,) {
  if (scaler.scale == 1) return block;
  let bTop = scaler.toDOM(block.top,), bBottom = scaler.toDOM(block.bottom,);
  return new BlockInfo(
    block.from,
    block.length,
    bTop,
    bBottom - bTop,
    Array.isArray(block._content,) ? block._content.map((b,) => scaleBlock(b, scaler,)) : block._content,
  );
}
var theme = /* @__PURE__ */ Facet.define({ combine: (strs,) => strs.join(' ',), },);
var darkTheme = /* @__PURE__ */ Facet.define({ combine: (values,) => values.indexOf(true,) > -1, },);
var baseThemeID = /* @__PURE__ */ StyleModule.newName();
var baseLightID = /* @__PURE__ */ StyleModule.newName();
var baseDarkID = /* @__PURE__ */ StyleModule.newName();
var lightDarkIDs = { '&light': '.' + baseLightID, '&dark': '.' + baseDarkID, };
function buildTheme(main, spec, scopes,) {
  return new StyleModule(spec, {
    finish(sel,) {
      return /&/.test(sel,)
        ? sel.replace(/&\w*/, (m,) => {
          if (m == '&') return main;
          if (!scopes || !scopes[m]) throw new RangeError(`Unsupported selector: ${m}`,);
          return scopes[m];
        },)
        : main + ' ' + sel;
    },
  },);
}
var baseTheme$1 = /* @__PURE__ */ buildTheme('.' + baseThemeID, {
  '&': {
    position: 'relative !important',
    boxSizing: 'border-box',
    '&.cm-focused': {
      // Provide a simple default outline to make sure a focused
      // editor is visually distinct. Can't leave the default behavior
      // because that will apply to the content element, which is
      // inside the scrollable container and doesn't include the
      // gutters. We also can't use an 'auto' outline, since those
      // are, for some reason, drawn behind the element content, which
      // will cause things like the active line background to cover
      // the outline (#297).
      outline: '1px dotted #212121',
    },
    display: 'flex !important',
    flexDirection: 'column',
  },
  '.cm-scroller': {
    display: 'flex !important',
    alignItems: 'flex-start !important',
    fontFamily: 'monospace',
    lineHeight: 1.4,
    height: '100%',
    overflowX: 'auto',
    position: 'relative',
    zIndex: 0,
  },
  '.cm-content': {
    margin: 0,
    flexGrow: 2,
    flexShrink: 0,
    display: 'block',
    whiteSpace: 'pre',
    wordWrap: 'normal',
    boxSizing: 'border-box',
    padding: '4px 0',
    outline: 'none',
    '&[contenteditable=true]': { WebkitUserModify: 'read-write-plaintext-only', },
  },
  '.cm-lineWrapping': {
    whiteSpace_fallback: 'pre-wrap',
    whiteSpace: 'break-spaces',
    wordBreak: 'break-word',
    overflowWrap: 'anywhere',
    flexShrink: 1,
  },
  '&light .cm-content': { caretColor: 'black', },
  '&dark .cm-content': { caretColor: 'white', },
  '.cm-line': { display: 'block', padding: '0 2px 0 6px', },
  '.cm-layer': { position: 'absolute', left: 0, top: 0, contain: 'size style', '& > *': { position: 'absolute', }, },
  '&light .cm-selectionBackground': { background: '#d9d9d9', },
  '&dark .cm-selectionBackground': { background: '#222', },
  '&light.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground': { background: '#d7d4f0', },
  '&dark.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground': { background: '#233', },
  '.cm-cursorLayer': { pointerEvents: 'none', },
  '&.cm-focused > .cm-scroller > .cm-cursorLayer': { animation: 'steps(1) cm-blink 1.2s infinite', },
  // Two animations defined so that we can switch between them to
  // restart the animation without forcing another style
  // recomputation.
  '@keyframes cm-blink': { '0%': {}, '50%': { opacity: 0, }, '100%': {}, },
  '@keyframes cm-blink2': { '0%': {}, '50%': { opacity: 0, }, '100%': {}, },
  '.cm-cursor, .cm-dropCursor': { borderLeft: '1.2px solid black', marginLeft: '-0.6px', pointerEvents: 'none', },
  '.cm-cursor': { display: 'none', },
  '&dark .cm-cursor': { borderLeftColor: '#444', },
  '.cm-dropCursor': { position: 'absolute', },
  '&.cm-focused > .cm-scroller > .cm-cursorLayer .cm-cursor': { display: 'block', },
  '&light .cm-activeLine': { backgroundColor: '#cceeff44', },
  '&dark .cm-activeLine': { backgroundColor: '#99eeff33', },
  '&light .cm-specialChar': { color: 'red', },
  '&dark .cm-specialChar': { color: '#f78', },
  '.cm-gutters': { flexShrink: 0, display: 'flex', height: '100%', boxSizing: 'border-box', left: 0, zIndex: 200, },
  '&light .cm-gutters': { backgroundColor: '#f5f5f5', color: '#6c6c6c', borderRight: '1px solid #ddd', },
  '&dark .cm-gutters': { backgroundColor: '#333338', color: '#ccc', },
  '.cm-gutter': {
    display: 'flex !important',
    flexDirection: 'column',
    flexShrink: 0,
    boxSizing: 'border-box',
    minHeight: '100%',
    overflow: 'hidden',
  },
  '.cm-gutterElement': { boxSizing: 'border-box', },
  '.cm-lineNumbers .cm-gutterElement': { padding: '0 3px 0 5px', minWidth: '20px', textAlign: 'right', whiteSpace: 'nowrap', },
  '&light .cm-activeLineGutter': { backgroundColor: '#e2f2ff', },
  '&dark .cm-activeLineGutter': { backgroundColor: '#222227', },
  '.cm-panels': { boxSizing: 'border-box', position: 'sticky', left: 0, right: 0, },
  '&light .cm-panels': { backgroundColor: '#f5f5f5', color: 'black', },
  '&light .cm-panels-top': { borderBottom: '1px solid #ddd', },
  '&light .cm-panels-bottom': { borderTop: '1px solid #ddd', },
  '&dark .cm-panels': { backgroundColor: '#333338', color: 'white', },
  '.cm-tab': { display: 'inline-block', overflow: 'hidden', verticalAlign: 'bottom', },
  '.cm-widgetBuffer': { verticalAlign: 'text-top', height: '1em', width: 0, display: 'inline', },
  '.cm-placeholder': { color: '#888', display: 'inline-block', verticalAlign: 'top', },
  '.cm-highlightSpace:before': { content: 'attr(data-display)', position: 'absolute', pointerEvents: 'none', color: '#888', },
  '.cm-highlightTab': {
    backgroundImage:
      `url('data:image/svg+xml,<svg xmlns="http://www.w3.org/2000/svg" width="200" height="20"><path stroke="%23888" stroke-width="1" fill="none" d="M1 10H196L190 5M190 15L196 10M197 4L197 16"/></svg>')`,
    backgroundSize: 'auto 100%',
    backgroundPosition: 'right 90%',
    backgroundRepeat: 'no-repeat',
  },
  '.cm-trailingSpace': { backgroundColor: '#ff332255', },
  '.cm-button': { verticalAlign: 'middle', color: 'inherit', fontSize: '70%', padding: '.2em 1em', borderRadius: '1px', },
  '&light .cm-button': {
    backgroundImage: 'linear-gradient(#eff1f5, #d9d9df)',
    border: '1px solid #888',
    '&:active': { backgroundImage: 'linear-gradient(#b4b4b4, #d0d3d6)', },
  },
  '&dark .cm-button': {
    backgroundImage: 'linear-gradient(#393939, #111)',
    border: '1px solid #888',
    '&:active': { backgroundImage: 'linear-gradient(#111, #333)', },
  },
  '.cm-textfield': { verticalAlign: 'middle', color: 'inherit', fontSize: '70%', border: '1px solid silver', padding: '.2em .5em', },
  '&light .cm-textfield': { backgroundColor: 'white', },
  '&dark .cm-textfield': { border: '1px solid #555', backgroundColor: 'inherit', },
}, lightDarkIDs,);
var DOMChange = class {
  constructor(view, start, end, typeOver,) {
    this.typeOver = typeOver;
    this.bounds = null;
    this.text = '';
    let { impreciseHead: iHead, impreciseAnchor: iAnchor, } = view.docView;
    if (view.state.readOnly && start > -1) {
      this.newSel = null;
    } else if (start > -1 && (this.bounds = view.docView.domBoundsAround(start, end, 0,))) {
      let selPoints = iHead || iAnchor ? [] : selectionPoints(view,);
      let reader = new DOMReader(selPoints, view.state,);
      reader.readRange(this.bounds.startDOM, this.bounds.endDOM,);
      this.text = reader.text;
      this.newSel = selectionFromPoints(selPoints, this.bounds.from,);
    } else {
      let domSel = view.observer.selectionRange;
      let head =
        iHead && iHead.node == domSel.focusNode && iHead.offset == domSel.focusOffset || !contains(view.contentDOM, domSel.focusNode,)
          ? view.state.selection.main.head
          : view.docView.posFromDOM(domSel.focusNode, domSel.focusOffset,);
      let anchor =
        iAnchor && iAnchor.node == domSel.anchorNode && iAnchor.offset == domSel.anchorOffset ||
          !contains(view.contentDOM, domSel.anchorNode,)
          ? view.state.selection.main.anchor
          : view.docView.posFromDOM(domSel.anchorNode, domSel.anchorOffset,);
      this.newSel = EditorSelection.single(anchor, head,);
    }
  }
};
function applyDOMChange(view, domChange,) {
  let change;
  let { newSel, } = domChange, sel = view.state.selection.main;
  let lastKey = view.inputState.lastKeyTime > Date.now() - 100 ? view.inputState.lastKeyCode : -1;
  if (domChange.bounds) {
    let { from, to, } = domChange.bounds;
    let preferredPos = sel.from, preferredSide = null;
    if (lastKey === 8 || browser.android && domChange.text.length < to - from) {
      preferredPos = sel.to;
      preferredSide = 'end';
    }
    let diff = findDiff(view.state.doc.sliceString(from, to, LineBreakPlaceholder,), domChange.text, preferredPos - from, preferredSide,);
    if (diff) {
      if (
        browser.chrome && lastKey == 13 && diff.toB == diff.from + 2 &&
        domChange.text.slice(diff.from, diff.toB,) == LineBreakPlaceholder + LineBreakPlaceholder
      ) diff.toB--;
      change = {
        from: from + diff.from,
        to: from + diff.toA,
        insert: Text.of(domChange.text.slice(diff.from, diff.toB,).split(LineBreakPlaceholder,),),
      };
    }
  } else if (newSel && (!view.hasFocus && view.state.facet(editable,) || newSel.main.eq(sel,))) {
    newSel = null;
  }
  if (!change && !newSel) return false;
  if (!change && domChange.typeOver && !sel.empty && newSel && newSel.main.empty) {
    change = { from: sel.from, to: sel.to, insert: view.state.doc.slice(sel.from, sel.to,), };
  } else if (
    change && change.from >= sel.from && change.to <= sel.to && (change.from != sel.from || change.to != sel.to) &&
    sel.to - sel.from - (change.to - change.from) <= 4
  ) {
    change = {
      from: sel.from,
      to: sel.to,
      insert: view.state.doc.slice(sel.from, change.from,).append(change.insert,).append(view.state.doc.slice(change.to, sel.to,),),
    };
  } else if (
    (browser.mac || browser.android) && change && change.from == change.to && change.from == sel.head - 1 &&
    /^\. ?$/.test(change.insert.toString(),) && view.contentDOM.getAttribute('autocorrect',) == 'off'
  ) {
    if (newSel && change.insert.length == 2) newSel = EditorSelection.single(newSel.main.anchor - 1, newSel.main.head - 1,);
    change = { from: sel.from, to: sel.to, insert: Text.of([' ',],), };
  } else if (
    browser.chrome && change && change.from == change.to && change.from == sel.head && change.insert.toString() == '\n ' &&
    view.lineWrapping
  ) {
    if (newSel) newSel = EditorSelection.single(newSel.main.anchor - 1, newSel.main.head - 1,);
    change = { from: sel.from, to: sel.to, insert: Text.of([' ',],), };
  }
  if (change) {
    let startState = view.state;
    if (browser.ios && view.inputState.flushIOSKey(view,)) return true;
    if (
      browser.android &&
      (change.from == sel.from && change.to == sel.to && change.insert.length == 1 && change.insert.lines == 2 &&
          dispatchKey(view.contentDOM, 'Enter', 13,) ||
        (change.from == sel.from - 1 && change.to == sel.to && change.insert.length == 0 ||
            lastKey == 8 && change.insert.length < change.to - change.from) && dispatchKey(view.contentDOM, 'Backspace', 8,) ||
        change.from == sel.from && change.to == sel.to + 1 && change.insert.length == 0 && dispatchKey(view.contentDOM, 'Delete', 46,))
    ) return true;
    let text = change.insert.toString();
    if (view.state.facet(inputHandler,).some((h,) => h(view, change.from, change.to, text,))) return true;
    if (view.inputState.composing >= 0) view.inputState.composing++;
    let tr;
    if (
      change.from >= sel.from && change.to <= sel.to && change.to - change.from >= (sel.to - sel.from) / 3 &&
      (!newSel || newSel.main.empty && newSel.main.from == change.from + change.insert.length) && view.inputState.composing < 0
    ) {
      let before = sel.from < change.from ? startState.sliceDoc(sel.from, change.from,) : '';
      let after = sel.to > change.to ? startState.sliceDoc(change.to, sel.to,) : '';
      tr = startState.replaceSelection(view.state.toText(before + change.insert.sliceString(0, void 0, view.state.lineBreak,) + after,),);
    } else {
      let changes = startState.changes(change,);
      let mainSel = newSel && newSel.main.to <= changes.newLength ? newSel.main : void 0;
      if (startState.selection.ranges.length > 1 && view.inputState.composing >= 0 && change.to <= sel.to && change.to >= sel.to - 10) {
        let replaced = view.state.sliceDoc(change.from, change.to,);
        let compositionRange = compositionSurroundingNode(view,) || view.state.doc.lineAt(sel.head,);
        let offset = sel.to - change.to, size = sel.to - sel.from;
        tr = startState.changeByRange((range,) => {
          if (range.from == sel.from && range.to == sel.to) return { changes, range: mainSel || range.map(changes,), };
          let to = range.to - offset, from = to - replaced.length;
          if (
            range.to - range.from != size || view.state.sliceDoc(from, to,) != replaced || // changes in the same node work without aborting
            // composition, so cursors in the composition range are
            // ignored.
            compositionRange && range.to >= compositionRange.from && range.from <= compositionRange.to
          ) return { range, };
          let rangeChanges = startState.changes({ from, to, insert: change.insert, },), selOff = range.to - sel.to;
          return {
            changes: rangeChanges,
            range: !mainSel
              ? range.map(rangeChanges,)
              : EditorSelection.range(Math.max(0, mainSel.anchor + selOff,), Math.max(0, mainSel.head + selOff,),),
          };
        },);
      } else {
        tr = { changes, selection: mainSel && startState.selection.replaceRange(mainSel,), };
      }
    }
    let userEvent = 'input.type';
    if (view.composing || view.inputState.compositionPendingChange && view.inputState.compositionEndedAt > Date.now() - 50) {
      view.inputState.compositionPendingChange = false;
      userEvent += '.compose';
      if (view.inputState.compositionFirstChange) {
        userEvent += '.start';
        view.inputState.compositionFirstChange = false;
      }
    }
    view.dispatch(tr, { scrollIntoView: true, userEvent, },);
    return true;
  } else if (newSel && !newSel.main.eq(sel,)) {
    let scrollIntoView2 = false, userEvent1 = 'select';
    if (view.inputState.lastSelectionTime > Date.now() - 50) {
      if (view.inputState.lastSelectionOrigin == 'select') scrollIntoView2 = true;
      userEvent1 = view.inputState.lastSelectionOrigin;
    }
    view.dispatch({ selection: newSel, scrollIntoView: scrollIntoView2, userEvent: userEvent1, },);
    return true;
  } else {
    return false;
  }
}
function findDiff(a, b, preferredPos, preferredSide,) {
  let minLen = Math.min(a.length, b.length,);
  let from = 0;
  while (from < minLen && a.charCodeAt(from,) == b.charCodeAt(from,)) from++;
  if (from == minLen && a.length == b.length) return null;
  let toA = a.length, toB = b.length;
  while (toA > 0 && toB > 0 && a.charCodeAt(toA - 1,) == b.charCodeAt(toB - 1,)) {
    toA--;
    toB--;
  }
  if (preferredSide == 'end') {
    let adjust = Math.max(0, from - Math.min(toA, toB,),);
    preferredPos -= toA + adjust - from;
  }
  if (toA < from && a.length < b.length) {
    let move = preferredPos <= from && preferredPos >= toA ? from - preferredPos : 0;
    from -= move;
    toB = from + (toB - toA);
    toA = from;
  } else if (toB < from) {
    let move1 = preferredPos <= from && preferredPos >= toB ? from - preferredPos : 0;
    from -= move1;
    toA = from + (toA - toB);
    toB = from;
  }
  return { from, toA, toB, };
}
function selectionPoints(view,) {
  let result = [];
  if (view.root.activeElement != view.contentDOM) return result;
  let { anchorNode, anchorOffset, focusNode, focusOffset, } = view.observer.selectionRange;
  if (anchorNode) {
    result.push(new DOMPoint(anchorNode, anchorOffset,),);
    if (focusNode != anchorNode || focusOffset != anchorOffset) result.push(new DOMPoint(focusNode, focusOffset,),);
  }
  return result;
}
function selectionFromPoints(points, base2,) {
  if (points.length == 0) return null;
  let anchor = points[0].pos, head = points.length == 2 ? points[1].pos : anchor;
  return anchor > -1 && head > -1 ? EditorSelection.single(anchor + base2, head + base2,) : null;
}
var observeOptions = { childList: true, characterData: true, subtree: true, attributes: true, characterDataOldValue: true, };
var useCharData = browser.ie && browser.ie_version <= 11;
var DOMObserver = class {
  onScrollChanged(e,) {
    this.view.inputState.runScrollHandlers(this.view, e,);
    if (this.intersecting) this.view.measure();
  }
  onScroll(e,) {
    if (this.intersecting) this.flush(false,);
    this.onScrollChanged(e,);
  }
  onResize() {
    if (this.resizeTimeout < 0) {
      this.resizeTimeout = setTimeout(() => {
        this.resizeTimeout = -1;
        this.view.requestMeasure();
      }, 50,);
    }
  }
  onPrint() {
    this.view.viewState.printing = true;
    this.view.measure();
    setTimeout(() => {
      this.view.viewState.printing = false;
      this.view.requestMeasure();
    }, 500,);
  }
  updateGaps(gaps,) {
    if (this.gapIntersection && (gaps.length != this.gaps.length || this.gaps.some((g, i2,) => g != gaps[i2]))) {
      this.gapIntersection.disconnect();
      for (let gap of gaps) this.gapIntersection.observe(gap,);
      this.gaps = gaps;
    }
  }
  onSelectionChange(event,) {
    let wasChanged = this.selectionChanged;
    if (!this.readSelectionRange() || this.delayedAndroidKey) return;
    let { view, } = this, sel = this.selectionRange;
    if (view.state.facet(editable,) ? view.root.activeElement != this.dom : !hasSelection(view.dom, sel,)) return;
    let context = sel.anchorNode && view.docView.nearest(sel.anchorNode,);
    if (context && context.ignoreEvent(event,)) {
      if (!wasChanged) this.selectionChanged = false;
      return;
    }
    if (
      (browser.ie && browser.ie_version <= 11 || browser.android && browser.chrome) && !view.state.selection.main.empty && sel.focusNode &&
      isEquivalentPosition(sel.focusNode, sel.focusOffset, sel.anchorNode, sel.anchorOffset,)
    ) this.flushSoon();
    else this.flush(false,);
  }
  readSelectionRange() {
    let { view, } = this;
    let range =
      browser.safari && view.root.nodeType == 11 && deepActiveElement(this.dom.ownerDocument,) == this.dom &&
        safariSelectionRangeHack(this.view,) || getSelection(view.root,);
    if (!range || this.selectionRange.eq(range,)) return false;
    let local = hasSelection(this.dom, range,);
    if (
      local && !this.selectionChanged && view.inputState.lastFocusTime > Date.now() - 200 &&
      view.inputState.lastTouchTime < Date.now() - 300 && atElementStart(this.dom, range,)
    ) {
      this.view.inputState.lastFocusTime = 0;
      view.docView.updateSelection();
      return false;
    }
    this.selectionRange.setRange(range,);
    if (local) this.selectionChanged = true;
    return true;
  }
  setSelectionRange(anchor, head,) {
    this.selectionRange.set(anchor.node, anchor.offset, head.node, head.offset,);
    this.selectionChanged = false;
  }
  clearSelectionRange() {
    this.selectionRange.set(null, 0, null, 0,);
  }
  listenForScroll() {
    this.parentCheck = -1;
    let i2 = 0, changed = null;
    for (let dom = this.dom; dom;) {
      if (dom.nodeType == 1) {
        if (!changed && i2 < this.scrollTargets.length && this.scrollTargets[i2] == dom) i2++;
        else if (!changed) changed = this.scrollTargets.slice(0, i2,);
        if (changed) changed.push(dom,);
        dom = dom.assignedSlot || dom.parentNode;
      } else if (dom.nodeType == 11) {
        dom = dom.host;
      } else {
        break;
      }
    }
    if (i2 < this.scrollTargets.length && !changed) changed = this.scrollTargets.slice(0, i2,);
    if (changed) {
      for (let dom1 of this.scrollTargets) dom1.removeEventListener('scroll', this.onScroll,);
      for (let dom2 of this.scrollTargets = changed) dom2.addEventListener('scroll', this.onScroll,);
    }
  }
  ignore(f,) {
    if (!this.active) return f();
    try {
      this.stop();
      return f();
    } finally {
      this.start();
      this.clear();
    }
  }
  start() {
    if (this.active) return;
    this.observer.observe(this.dom, observeOptions,);
    if (useCharData) this.dom.addEventListener('DOMCharacterDataModified', this.onCharData,);
    this.active = true;
  }
  stop() {
    if (!this.active) return;
    this.active = false;
    this.observer.disconnect();
    if (useCharData) this.dom.removeEventListener('DOMCharacterDataModified', this.onCharData,);
  }
  // Throw away any pending changes
  clear() {
    this.processRecords();
    this.queue.length = 0;
    this.selectionChanged = false;
  }
  // Chrome Android, especially in combination with GBoard, not only
  // doesn't reliably fire regular key events, but also often
  // surrounds the effect of enter or backspace with a bunch of
  // composition events that, when interrupted, cause text duplication
  // or other kinds of corruption. This hack makes the editor back off
  // from handling DOM changes for a moment when such a key is
  // detected (via beforeinput or keydown), and then tries to flush
  // them or, if that has no effect, dispatches the given key.
  delayAndroidKey(key, keyCode,) {
    var _a2;
    if (!this.delayedAndroidKey) {
      let flush = () => {
        let key2 = this.delayedAndroidKey;
        if (key2) {
          this.clearDelayedAndroidKey();
          this.view.inputState.lastKeyCode = key2.keyCode;
          this.view.inputState.lastKeyTime = Date.now();
          let flushed = this.flush();
          if (!flushed && key2.force) dispatchKey(this.dom, key2.key, key2.keyCode,);
        }
      };
      this.flushingAndroidKey = this.view.win.requestAnimationFrame(flush,);
    }
    if (!this.delayedAndroidKey || key == 'Enter') {
      this.delayedAndroidKey = {
        key,
        keyCode,
        // Only run the key handler when no changes are detected if
        // this isn't coming right after another change, in which case
        // it is probably part of a weird chain of updates, and should
        // be ignored if it returns the DOM to its previous state.
        force: this.lastChange < Date.now() - 50 || !!((_a2 = this.delayedAndroidKey) === null || _a2 === void 0 ? void 0 : _a2.force),
      };
    }
  }
  clearDelayedAndroidKey() {
    this.win.cancelAnimationFrame(this.flushingAndroidKey,);
    this.delayedAndroidKey = null;
    this.flushingAndroidKey = -1;
  }
  flushSoon() {
    if (this.delayedFlush < 0) {
      this.delayedFlush = this.view.win.requestAnimationFrame(() => {
        this.delayedFlush = -1;
        this.flush();
      },);
    }
  }
  forceFlush() {
    if (this.delayedFlush >= 0) {
      this.view.win.cancelAnimationFrame(this.delayedFlush,);
      this.delayedFlush = -1;
    }
    this.flush();
  }
  pendingRecords() {
    for (let mut of this.observer.takeRecords()) this.queue.push(mut,);
    return this.queue;
  }
  processRecords() {
    let records = this.pendingRecords();
    if (records.length) this.queue = [];
    let from = -1, to = -1, typeOver = false;
    for (let record of records) {
      let range = this.readMutation(record,);
      if (!range) continue;
      if (range.typeOver) typeOver = true;
      if (from == -1) {
        ({ from, to, } = range);
      } else {
        from = Math.min(range.from, from,);
        to = Math.max(range.to, to,);
      }
    }
    return { from, to, typeOver, };
  }
  readChange() {
    let { from, to, typeOver, } = this.processRecords();
    let newSel = this.selectionChanged && hasSelection(this.dom, this.selectionRange,);
    if (from < 0 && !newSel) return null;
    if (from > -1) this.lastChange = Date.now();
    this.view.inputState.lastFocusTime = 0;
    this.selectionChanged = false;
    return new DOMChange(this.view, from, to, typeOver,);
  }
  // Apply pending changes, if any
  flush(readSelection = true,) {
    if (this.delayedFlush >= 0 || this.delayedAndroidKey) return false;
    if (readSelection) this.readSelectionRange();
    let domChange = this.readChange();
    if (!domChange) return false;
    let startState = this.view.state;
    let handled = applyDOMChange(this.view, domChange,);
    if (this.view.state == startState) this.view.update([],);
    return handled;
  }
  readMutation(rec,) {
    let cView = this.view.docView.nearest(rec.target,);
    if (!cView || cView.ignoreMutation(rec,)) return null;
    cView.markDirty(rec.type == 'attributes',);
    if (rec.type == 'attributes') cView.dirty |= 4;
    if (rec.type == 'childList') {
      let childBefore = findChild(cView, rec.previousSibling || rec.target.previousSibling, -1,);
      let childAfter = findChild(cView, rec.nextSibling || rec.target.nextSibling, 1,);
      return {
        from: childBefore ? cView.posAfter(childBefore,) : cView.posAtStart,
        to: childAfter ? cView.posBefore(childAfter,) : cView.posAtEnd,
        typeOver: false,
      };
    } else if (rec.type == 'characterData') {
      return { from: cView.posAtStart, to: cView.posAtEnd, typeOver: rec.target.nodeValue == rec.oldValue, };
    } else {
      return null;
    }
  }
  setWindow(win,) {
    if (win != this.win) {
      this.removeWindowListeners(this.win,);
      this.win = win;
      this.addWindowListeners(this.win,);
    }
  }
  addWindowListeners(win,) {
    win.addEventListener('resize', this.onResize,);
    win.addEventListener('beforeprint', this.onPrint,);
    win.addEventListener('scroll', this.onScroll,);
    win.document.addEventListener('selectionchange', this.onSelectionChange,);
  }
  removeWindowListeners(win,) {
    win.removeEventListener('scroll', this.onScroll,);
    win.removeEventListener('resize', this.onResize,);
    win.removeEventListener('beforeprint', this.onPrint,);
    win.document.removeEventListener('selectionchange', this.onSelectionChange,);
  }
  destroy() {
    var _a2, _b, _c, _d;
    this.stop();
    (_a2 = this.intersection) === null || _a2 === void 0 ? void 0 : _a2.disconnect();
    (_b = this.gapIntersection) === null || _b === void 0 ? void 0 : _b.disconnect();
    (_c = this.resizeScroll) === null || _c === void 0 ? void 0 : _c.disconnect();
    (_d = this.resizeContent) === null || _d === void 0 ? void 0 : _d.disconnect();
    for (let dom of this.scrollTargets) dom.removeEventListener('scroll', this.onScroll,);
    this.removeWindowListeners(this.win,);
    clearTimeout(this.parentCheck,);
    clearTimeout(this.resizeTimeout,);
    this.win.cancelAnimationFrame(this.delayedFlush,);
    this.win.cancelAnimationFrame(this.flushingAndroidKey,);
  }
  constructor(view,) {
    this.view = view;
    this.active = false;
    this.selectionRange = new DOMSelectionState();
    this.selectionChanged = false;
    this.delayedFlush = -1;
    this.resizeTimeout = -1;
    this.queue = [];
    this.delayedAndroidKey = null;
    this.flushingAndroidKey = -1;
    this.lastChange = 0;
    this.scrollTargets = [];
    this.intersection = null;
    this.resizeScroll = null;
    this.resizeContent = null;
    this.intersecting = false;
    this.gapIntersection = null;
    this.gaps = [];
    this.parentCheck = -1;
    this.dom = view.contentDOM;
    this.observer = new MutationObserver((mutations,) => {
      for (let mut of mutations) this.queue.push(mut,);
      if (
        (browser.ie && browser.ie_version <= 11 || browser.ios && view.composing) && mutations.some((m,) =>
          m.type == 'childList' && m.removedNodes.length || m.type == 'characterData' && m.oldValue.length > m.target.nodeValue.length
        )
      ) this.flushSoon();
      else this.flush();
    },);
    if (useCharData) {
      this.onCharData = (event,) => {
        this.queue.push({ target: event.target, type: 'characterData', oldValue: event.prevValue, },);
        this.flushSoon();
      };
    }
    this.onSelectionChange = this.onSelectionChange.bind(this,);
    this.onResize = this.onResize.bind(this,);
    this.onPrint = this.onPrint.bind(this,);
    this.onScroll = this.onScroll.bind(this,);
    if (typeof ResizeObserver == 'function') {
      this.resizeScroll = new ResizeObserver(() => {
        var _a2;
        if (((_a2 = this.view.docView) === null || _a2 === void 0 ? void 0 : _a2.lastUpdate) < Date.now() - 75) this.onResize();
      },);
      this.resizeScroll.observe(view.scrollDOM,);
      this.resizeContent = new ResizeObserver(() => this.view.requestMeasure());
      this.resizeContent.observe(view.contentDOM,);
    }
    this.addWindowListeners(this.win = view.win,);
    this.start();
    if (typeof IntersectionObserver == 'function') {
      this.intersection = new IntersectionObserver((entries,) => {
        if (this.parentCheck < 0) this.parentCheck = setTimeout(this.listenForScroll.bind(this,), 1e3,);
        if (entries.length > 0 && entries[entries.length - 1].intersectionRatio > 0 != this.intersecting) {
          this.intersecting = !this.intersecting;
          if (this.intersecting != this.view.inView) this.onScrollChanged(document.createEvent('Event',),);
        }
      }, { threshold: [0, 1e-3,], },);
      this.intersection.observe(this.dom,);
      this.gapIntersection = new IntersectionObserver((entries,) => {
        if (entries.length > 0 && entries[entries.length - 1].intersectionRatio > 0) this.onScrollChanged(document.createEvent('Event',),);
      }, {},);
    }
    this.listenForScroll();
    this.readSelectionRange();
  }
};
function findChild(cView, dom, dir,) {
  while (dom) {
    let curView = ContentView.get(dom,);
    if (curView && curView.parent == cView) return curView;
    let parent = dom.parentNode;
    dom = parent != cView.dom ? parent : dir > 0 ? dom.nextSibling : dom.previousSibling;
  }
  return null;
}
function safariSelectionRangeHack(view,) {
  let found = null;
  function read(event,) {
    event.preventDefault();
    event.stopImmediatePropagation();
    found = event.getTargetRanges()[0];
  }
  view.contentDOM.addEventListener('beforeinput', read, true,);
  view.dom.ownerDocument.execCommand('indent',);
  view.contentDOM.removeEventListener('beforeinput', read, true,);
  if (!found) return null;
  let anchorNode = found.startContainer, anchorOffset = found.startOffset;
  let focusNode = found.endContainer, focusOffset = found.endOffset;
  let curAnchor = view.docView.domAtPos(view.state.selection.main.anchor,);
  if (isEquivalentPosition(curAnchor.node, curAnchor.offset, focusNode, focusOffset,)) {
    [anchorNode, anchorOffset, focusNode, focusOffset,] = [focusNode, focusOffset, anchorNode, anchorOffset,];
  }
  return { anchorNode, anchorOffset, focusNode, focusOffset, };
}
var EditorView = class {
  /**
  The current editor state.
  */
  get state() {
    return this.viewState.state;
  }
  /**
  To be able to display large documents without consuming too much
  memory or overloading the browser, CodeMirror only draws the
  code that is visible (plus a margin around it) to the DOM. This
  property tells you the extent of the current drawn viewport, in
  document positions.
  */
  get viewport() {
    return this.viewState.viewport;
  }
  /**
  When there are, for example, large collapsed ranges in the
  viewport, its size can be a lot bigger than the actual visible
  content. Thus, if you are doing something like styling the
  content in the viewport, it is preferable to only do so for
  these ranges, which are the subset of the viewport that is
  actually drawn.
  */
  get visibleRanges() {
    return this.viewState.visibleRanges;
  }
  /**
  Returns false when the editor is entirely scrolled out of view
  or otherwise hidden.
  */
  get inView() {
    return this.viewState.inView;
  }
  /**
  Indicates whether the user is currently composing text via
  [IME](https://en.wikipedia.org/wiki/Input_method), and at least
  one change has been made in the current composition.
  */
  get composing() {
    return this.inputState.composing > 0;
  }
  /**
  Indicates whether the user is currently in composing state. Note
  that on some platforms, like Android, this will be the case a
  lot, since just putting the cursor on a word starts a
  composition there.
  */
  get compositionStarted() {
    return this.inputState.composing >= 0;
  }
  /**
  The document or shadow root that the view lives in.
  */
  get root() {
    return this._root;
  }
  /**
  @internal
  */
  get win() {
    return this.dom.ownerDocument.defaultView || window;
  }
  dispatch(...input) {
    let tr = input.length == 1 && input[0] instanceof Transaction ? input[0] : this.state.update(...input,);
    this._dispatch(tr, this,);
  }
  /**
  Update the view for the given array of transactions. This will
  update the visible document and selection to match the state
  produced by the transactions, and notify view plugins of the
  change. You should usually call
  [`dispatch`](https://codemirror.net/6/docs/ref/#view.EditorView.dispatch) instead, which uses this
  as a primitive.
  */
  update(transactions,) {
    if (this.updateState != 0) throw new Error('Calls to EditorView.update are not allowed while an update is in progress',);
    let redrawn = false, attrsChanged = false, update;
    let state = this.state;
    for (let tr of transactions) {
      if (tr.startState != state) {
        throw new RangeError('Trying to update state with a transaction that doesn\'t start from the previous state.',);
      }
      state = tr.state;
    }
    if (this.destroyed) {
      this.viewState.state = state;
      return;
    }
    let focus = this.hasFocus, focusFlag = 0, dispatchFocus = null;
    if (transactions.some((tr,) => tr.annotation(isFocusChange,))) {
      this.inputState.notifiedFocused = focus;
      focusFlag = 1;
    } else if (focus != this.inputState.notifiedFocused) {
      this.inputState.notifiedFocused = focus;
      dispatchFocus = focusChangeTransaction(state, focus,);
      if (!dispatchFocus) focusFlag = 1;
    }
    let pendingKey = this.observer.delayedAndroidKey, domChange = null;
    if (pendingKey) {
      this.observer.clearDelayedAndroidKey();
      domChange = this.observer.readChange();
      if (domChange && !this.state.doc.eq(state.doc,) || !this.state.selection.eq(state.selection,)) domChange = null;
    } else {
      this.observer.clear();
    }
    if (state.facet(EditorState.phrases,) != this.state.facet(EditorState.phrases,)) return this.setState(state,);
    update = ViewUpdate.create(this, state, transactions,);
    update.flags |= focusFlag;
    let scrollTarget = this.viewState.scrollTarget;
    try {
      this.updateState = 2;
      for (let tr1 of transactions) {
        if (scrollTarget) scrollTarget = scrollTarget.map(tr1.changes,);
        if (tr1.scrollIntoView) {
          let { main, } = tr1.state.selection;
          scrollTarget = new ScrollTarget(main.empty ? main : EditorSelection.cursor(main.head, main.head > main.anchor ? -1 : 1,),);
        }
        for (let e of tr1.effects) if (e.is(scrollIntoView,)) scrollTarget = e.value;
      }
      this.viewState.update(update, scrollTarget,);
      this.bidiCache = CachedOrder.update(this.bidiCache, update.changes,);
      if (!update.empty) {
        this.updatePlugins(update,);
        this.inputState.update(update,);
      }
      redrawn = this.docView.update(update,);
      if (this.state.facet(styleModule,) != this.styleModules) this.mountStyles();
      attrsChanged = this.updateAttrs();
      this.showAnnouncements(transactions,);
      this.docView.updateSelection(redrawn, transactions.some((tr,) => tr.isUserEvent('select.pointer',)),);
    } finally {
      this.updateState = 0;
    }
    if (update.startState.facet(theme,) != update.state.facet(theme,)) this.viewState.mustMeasureContent = true;
    if (redrawn || attrsChanged || scrollTarget || this.viewState.mustEnforceCursorAssoc || this.viewState.mustMeasureContent) {
      this.requestMeasure();
    }
    if (!update.empty) for (let listener of this.state.facet(updateListener,)) listener(update,);
    if (dispatchFocus || domChange) {
      Promise.resolve().then(() => {
        if (dispatchFocus && this.state == dispatchFocus.startState) this.dispatch(dispatchFocus,);
        if (domChange) {
          if (!applyDOMChange(this, domChange,) && pendingKey.force) dispatchKey(this.contentDOM, pendingKey.key, pendingKey.keyCode,);
        }
      },);
    }
  }
  /**
  Reset the view to the given state. (This will cause the entire
  document to be redrawn and all view plugins to be reinitialized,
  so you should probably only use it when the new state isn't
  derived from the old state. Otherwise, use
  [`dispatch`](https://codemirror.net/6/docs/ref/#view.EditorView.dispatch) instead.)
  */
  setState(newState,) {
    if (this.updateState != 0) throw new Error('Calls to EditorView.setState are not allowed while an update is in progress',);
    if (this.destroyed) {
      this.viewState.state = newState;
      return;
    }
    this.updateState = 2;
    let hadFocus = this.hasFocus;
    try {
      for (let plugin2 of this.plugins) plugin2.destroy(this,);
      this.viewState = new ViewState(newState,);
      this.plugins = newState.facet(viewPlugin,).map((spec,) => new PluginInstance(spec,));
      this.pluginMap.clear();
      for (let plugin21 of this.plugins) plugin21.update(this,);
      this.docView = new DocView(this,);
      this.inputState.ensureHandlers(this, this.plugins,);
      this.mountStyles();
      this.updateAttrs();
      this.bidiCache = [];
    } finally {
      this.updateState = 0;
    }
    if (hadFocus) this.focus();
    this.requestMeasure();
  }
  updatePlugins(update,) {
    let prevSpecs = update.startState.facet(viewPlugin,), specs = update.state.facet(viewPlugin,);
    if (prevSpecs != specs) {
      let newPlugins = [];
      for (let spec of specs) {
        let found = prevSpecs.indexOf(spec,);
        if (found < 0) {
          newPlugins.push(new PluginInstance(spec,),);
        } else {
          let plugin2 = this.plugins[found];
          plugin2.mustUpdate = update;
          newPlugins.push(plugin2,);
        }
      }
      for (let plugin21 of this.plugins) if (plugin21.mustUpdate != update) plugin21.destroy(this,);
      this.plugins = newPlugins;
      this.pluginMap.clear();
      this.inputState.ensureHandlers(this, this.plugins,);
    } else {
      for (let p of this.plugins) p.mustUpdate = update;
    }
    for (let i2 = 0; i2 < this.plugins.length; i2++) this.plugins[i2].update(this,);
  }
  /**
  @internal
  */
  measure(flush = true,) {
    if (this.destroyed) return;
    if (this.measureScheduled > -1) this.win.cancelAnimationFrame(this.measureScheduled,);
    this.measureScheduled = 0;
    if (flush) this.observer.forceFlush();
    let updated = null;
    let sDOM = this.scrollDOM, { scrollAnchorPos, scrollAnchorHeight, } = this.viewState;
    this.viewState.scrollAnchorHeight = -1;
    if (scrollAnchorHeight < 0 || sDOM.scrollTop != this.viewState.scrollTop) {
      if (sDOM.scrollTop > sDOM.scrollHeight - sDOM.clientHeight - 4) {
        scrollAnchorPos = -1;
        scrollAnchorHeight = this.viewState.heightMap.height;
      } else {
        let block = this.viewState.lineBlockAtHeight(sDOM.scrollTop,);
        scrollAnchorPos = block.from;
        scrollAnchorHeight = block.top;
      }
    }
    try {
      for (let i2 = 0;; i2++) {
        this.updateState = 1;
        let oldViewport = this.viewport;
        let changed = this.viewState.measure(this,);
        if (!changed && !this.measureRequests.length && this.viewState.scrollTarget == null) break;
        if (i2 > 5) {
          console.warn(this.measureRequests.length ? 'Measure loop restarted more than 5 times' : 'Viewport failed to stabilize',);
          break;
        }
        let measuring = [];
        if (!(changed & 4)) [this.measureRequests, measuring,] = [measuring, this.measureRequests,];
        let measured = measuring.map((m,) => {
          try {
            return m.read(this,);
          } catch (e) {
            logException(this.state, e,);
            return BadMeasure;
          }
        },);
        let update = ViewUpdate.create(this, this.state, [],), redrawn = false, scrolled = false;
        update.flags |= changed;
        if (!updated) updated = update;
        else updated.flags |= changed;
        this.updateState = 2;
        if (!update.empty) {
          this.updatePlugins(update,);
          this.inputState.update(update,);
          this.updateAttrs();
          redrawn = this.docView.update(update,);
        }
        for (let i22 = 0; i22 < measuring.length; i22++) {
          if (measured[i22] != BadMeasure) {
            try {
              let m = measuring[i22];
              if (m.write) m.write(measured[i22], this,);
            } catch (e) {
              logException(this.state, e,);
            }
          }
        }
        if (this.viewState.editorHeight) {
          if (this.viewState.scrollTarget) {
            this.docView.scrollIntoView(this.viewState.scrollTarget,);
            this.viewState.scrollTarget = null;
            scrolled = true;
          } else if (scrollAnchorHeight > -1) {
            let newAnchorHeight = scrollAnchorPos < 0 ? this.viewState.heightMap.height : this.viewState.lineBlockAt(scrollAnchorPos,).top;
            let diff = newAnchorHeight - scrollAnchorHeight;
            if (diff > 1 || diff < -1) {
              sDOM.scrollTop += diff;
              scrolled = true;
            }
          }
        }
        if (redrawn) this.docView.updateSelection(true,);
        if (this.viewport.from == oldViewport.from && this.viewport.to == oldViewport.to && !scrolled && this.measureRequests.length == 0) {
          break;
        }
        scrollAnchorHeight = -1;
      }
    } finally {
      this.updateState = 0;
      this.measureScheduled = -1;
    }
    if (updated && !updated.empty) for (let listener of this.state.facet(updateListener,)) listener(updated,);
  }
  /**
  Get the CSS classes for the currently active editor themes.
  */
  get themeClasses() {
    return baseThemeID + ' ' + (this.state.facet(darkTheme,) ? baseDarkID : baseLightID) + ' ' + this.state.facet(theme,);
  }
  updateAttrs() {
    let editorAttrs = attrsFromFacet(this, editorAttributes, {
      class: 'cm-editor' + (this.hasFocus ? ' cm-focused ' : ' ') + this.themeClasses,
    },);
    let contentAttrs = {
      spellcheck: 'false',
      autocorrect: 'off',
      autocapitalize: 'off',
      translate: 'no',
      contenteditable: !this.state.facet(editable,) ? 'false' : 'true',
      class: 'cm-content',
      style: `${browser.tabSize}: ${this.state.tabSize}`,
      role: 'textbox',
      'aria-multiline': 'true',
    };
    if (this.state.readOnly) contentAttrs['aria-readonly'] = 'true';
    attrsFromFacet(this, contentAttributes, contentAttrs,);
    let changed = this.observer.ignore(() => {
      let changedContent = updateAttrs(this.contentDOM, this.contentAttrs, contentAttrs,);
      let changedEditor = updateAttrs(this.dom, this.editorAttrs, editorAttrs,);
      return changedContent || changedEditor;
    },);
    this.editorAttrs = editorAttrs;
    this.contentAttrs = contentAttrs;
    return changed;
  }
  showAnnouncements(trs,) {
    let first = true;
    for (let tr of trs) {
      for (let effect of tr.effects) {
        if (effect.is(EditorView.announce,)) {
          if (first) this.announceDOM.textContent = '';
          first = false;
          let div = this.announceDOM.appendChild(document.createElement('div',),);
          div.textContent = effect.value;
        }
      }
    }
  }
  mountStyles() {
    this.styleModules = this.state.facet(styleModule,);
    StyleModule.mount(this.root, this.styleModules.concat(baseTheme$1,).reverse(),);
  }
  readMeasured() {
    if (this.updateState == 2) throw new Error('Reading the editor layout isn\'t allowed during an update',);
    if (this.updateState == 0 && this.measureScheduled > -1) this.measure(false,);
  }
  /**
  Schedule a layout measurement, optionally providing callbacks to
  do custom DOM measuring followed by a DOM write phase. Using
  this is preferable reading DOM layout directly from, for
  example, an event handler, because it'll make sure measuring and
  drawing done by other components is synchronized, avoiding
  unnecessary DOM layout computations.
  */
  requestMeasure(request,) {
    if (this.measureScheduled < 0) this.measureScheduled = this.win.requestAnimationFrame(() => this.measure());
    if (request) {
      if (this.measureRequests.indexOf(request,) > -1) return;
      if (request.key != null) {
        for (let i2 = 0; i2 < this.measureRequests.length; i2++) {
          if (this.measureRequests[i2].key === request.key) {
            this.measureRequests[i2] = request;
            return;
          }
        }
      }
      this.measureRequests.push(request,);
    }
  }
  /**
  Get the value of a specific plugin, if present. Note that
  plugins that crash can be dropped from a view, so even when you
  know you registered a given plugin, it is recommended to check
  the return value of this method.
  */
  plugin(plugin2,) {
    let known = this.pluginMap.get(plugin2,);
    if (known === void 0 || known && known.spec != plugin2) {
      this.pluginMap.set(
        plugin2,
        known = this.plugins.find((p,) => p.spec == plugin2) || null,
      );
    }
    return known && known.update(this,).value;
  }
  /**
  The top position of the document, in screen coordinates. This
  may be negative when the editor is scrolled down. Points
  directly to the top of the first line, not above the padding.
  */
  get documentTop() {
    return this.contentDOM.getBoundingClientRect().top + this.viewState.paddingTop;
  }
  /**
  Reports the padding above and below the document.
  */
  get documentPadding() {
    return { top: this.viewState.paddingTop, bottom: this.viewState.paddingBottom, };
  }
  /**
  Find the text line or block widget at the given vertical
  position (which is interpreted as relative to the [top of the
  document](https://codemirror.net/6/docs/ref/#view.EditorView.documentTop)).
  */
  elementAtHeight(height,) {
    this.readMeasured();
    return this.viewState.elementAtHeight(height,);
  }
  /**
  Find the line block (see
  [`lineBlockAt`](https://codemirror.net/6/docs/ref/#view.EditorView.lineBlockAt) at the given
  height, again interpreted relative to the [top of the
  document](https://codemirror.net/6/docs/ref/#view.EditorView.documentTop).
  */
  lineBlockAtHeight(height,) {
    this.readMeasured();
    return this.viewState.lineBlockAtHeight(height,);
  }
  /**
  Get the extent and vertical position of all [line
  blocks](https://codemirror.net/6/docs/ref/#view.EditorView.lineBlockAt) in the viewport. Positions
  are relative to the [top of the
  document](https://codemirror.net/6/docs/ref/#view.EditorView.documentTop);
  */
  get viewportLineBlocks() {
    return this.viewState.viewportLines;
  }
  /**
  Find the line block around the given document position. A line
  block is a range delimited on both sides by either a
  non-[hidden](https://codemirror.net/6/docs/ref/#view.Decoration^replace) line breaks, or the
  start/end of the document. It will usually just hold a line of
  text, but may be broken into multiple textblocks by block
  widgets.
  */
  lineBlockAt(pos,) {
    return this.viewState.lineBlockAt(pos,);
  }
  /**
  The editor's total content height.
  */
  get contentHeight() {
    return this.viewState.contentHeight;
  }
  /**
  Move a cursor position by [grapheme
  cluster](https://codemirror.net/6/docs/ref/#state.findClusterBreak). `forward` determines whether
  the motion is away from the line start, or towards it. In
  bidirectional text, the line is traversed in visual order, using
  the editor's [text direction](https://codemirror.net/6/docs/ref/#view.EditorView.textDirection).
  When the start position was the last one on the line, the
  returned position will be across the line break. If there is no
  further line, the original position is returned.

  By default, this method moves over a single cluster. The
  optional `by` argument can be used to move across more. It will
  be called with the first cluster as argument, and should return
  a predicate that determines, for each subsequent cluster,
  whether it should also be moved over.
  */
  moveByChar(start, forward, by,) {
    return skipAtoms(this, start, moveByChar(this, start, forward, by,),);
  }
  /**
  Move a cursor position across the next group of either
  [letters](https://codemirror.net/6/docs/ref/#state.EditorState.charCategorizer) or non-letter
  non-whitespace characters.
  */
  moveByGroup(start, forward,) {
    return skipAtoms(this, start, moveByChar(this, start, forward, (initial,) => byGroup(this, start.head, initial,),),);
  }
  /**
  Move to the next line boundary in the given direction. If
  `includeWrap` is true, line wrapping is on, and there is a
  further wrap point on the current line, the wrap point will be
  returned. Otherwise this function will return the start or end
  of the line.
  */
  moveToLineBoundary(start, forward, includeWrap = true,) {
    return moveToLineBoundary(this, start, forward, includeWrap,);
  }
  /**
  Move a cursor position vertically. When `distance` isn't given,
  it defaults to moving to the next line (including wrapped
  lines). Otherwise, `distance` should provide a positive distance
  in pixels.

  When `start` has a
  [`goalColumn`](https://codemirror.net/6/docs/ref/#state.SelectionRange.goalColumn), the vertical
  motion will use that as a target horizontal position. Otherwise,
  the cursor's own horizontal position is used. The returned
  cursor will have its goal column set to whichever column was
  used.
  */
  moveVertically(start, forward, distance,) {
    return skipAtoms(this, start, moveVertically(this, start, forward, distance,),);
  }
  /**
  Find the DOM parent node and offset (child offset if `node` is
  an element, character offset when it is a text node) at the
  given document position.

  Note that for positions that aren't currently in
  `visibleRanges`, the resulting DOM position isn't necessarily
  meaningful (it may just point before or after a placeholder
  element).
  */
  domAtPos(pos,) {
    return this.docView.domAtPos(pos,);
  }
  /**
  Find the document position at the given DOM node. Can be useful
  for associating positions with DOM events. Will raise an error
  when `node` isn't part of the editor content.
  */
  posAtDOM(node, offset = 0,) {
    return this.docView.posFromDOM(node, offset,);
  }
  posAtCoords(coords, precise = true,) {
    this.readMeasured();
    return posAtCoords(this, coords, precise,);
  }
  /**
  Get the screen coordinates at the given document position.
  `side` determines whether the coordinates are based on the
  element before (-1) or after (1) the position (if no element is
  available on the given side, the method will transparently use
  another strategy to get reasonable coordinates).
  */
  coordsAtPos(pos, side = 1,) {
    this.readMeasured();
    let rect = this.docView.coordsAt(pos, side,);
    if (!rect || rect.left == rect.right) return rect;
    let line = this.state.doc.lineAt(pos,), order = this.bidiSpans(line,);
    let span = order[BidiSpan.find(order, pos - line.from, -1, side,)];
    return flattenRect(rect, span.dir == Direction.LTR == side > 0,);
  }
  /**
  The default width of a character in the editor. May not
  accurately reflect the width of all characters (given variable
  width fonts or styling of invididual ranges).
  */
  get defaultCharacterWidth() {
    return this.viewState.heightOracle.charWidth;
  }
  /**
  The default height of a line in the editor. May not be accurate
  for all lines.
  */
  get defaultLineHeight() {
    return this.viewState.heightOracle.lineHeight;
  }
  /**
  The text direction
  ([`direction`](https://developer.mozilla.org/en-US/docs/Web/CSS/direction)
  CSS property) of the editor's content element.
  */
  get textDirection() {
    return this.viewState.defaultTextDirection;
  }
  /**
  Find the text direction of the block at the given position, as
  assigned by CSS. If
  [`perLineTextDirection`](https://codemirror.net/6/docs/ref/#view.EditorView^perLineTextDirection)
  isn't enabled, or the given position is outside of the viewport,
  this will always return the same as
  [`textDirection`](https://codemirror.net/6/docs/ref/#view.EditorView.textDirection). Note that
  this may trigger a DOM layout.
  */
  textDirectionAt(pos,) {
    let perLine = this.state.facet(perLineTextDirection,);
    if (!perLine || pos < this.viewport.from || pos > this.viewport.to) return this.textDirection;
    this.readMeasured();
    return this.docView.textDirectionAt(pos,);
  }
  /**
  Whether this editor [wraps lines](https://codemirror.net/6/docs/ref/#view.EditorView.lineWrapping)
  (as determined by the
  [`white-space`](https://developer.mozilla.org/en-US/docs/Web/CSS/white-space)
  CSS property of its content element).
  */
  get lineWrapping() {
    return this.viewState.heightOracle.lineWrapping;
  }
  /**
  Returns the bidirectional text structure of the given line
  (which should be in the current document) as an array of span
  objects. The order of these spans matches the [text
  direction](https://codemirror.net/6/docs/ref/#view.EditorView.textDirection)—if that is
  left-to-right, the leftmost spans come first, otherwise the
  rightmost spans come first.
  */
  bidiSpans(line,) {
    if (line.length > MaxBidiLine) return trivialOrder(line.length,);
    let dir = this.textDirectionAt(line.from,);
    for (let entry of this.bidiCache) if (entry.from == line.from && entry.dir == dir) return entry.order;
    let order = computeOrder(line.text, dir,);
    this.bidiCache.push(new CachedOrder(line.from, line.to, dir, order,),);
    return order;
  }
  /**
  Check whether the editor has focus.
  */
  get hasFocus() {
    var _a2;
    return (this.dom.ownerDocument.hasFocus() ||
      browser.safari && ((_a2 = this.inputState) === null || _a2 === void 0 ? void 0 : _a2.lastContextMenu) > Date.now() - 3e4) &&
      this.root.activeElement == this.contentDOM;
  }
  /**
  Put focus on the editor.
  */
  focus() {
    this.observer.ignore(() => {
      focusPreventScroll(this.contentDOM,);
      this.docView.updateSelection();
    },);
  }
  /**
  Update the [root](https://codemirror.net/6/docs/ref/##view.EditorViewConfig.root) in which the editor lives. This is only
  necessary when moving the editor's existing DOM to a new window or shadow root.
  */
  setRoot(root,) {
    if (this._root != root) {
      this._root = root;
      this.observer.setWindow((root.nodeType == 9 ? root : root.ownerDocument).defaultView || window,);
      this.mountStyles();
    }
  }
  /**
  Clean up this editor view, removing its element from the
  document, unregistering event handlers, and notifying
  plugins. The view instance can no longer be used after
  calling this.
  */
  destroy() {
    for (let plugin2 of this.plugins) plugin2.destroy(this,);
    this.plugins = [];
    this.inputState.destroy();
    this.dom.remove();
    this.observer.destroy();
    if (this.measureScheduled > -1) this.win.cancelAnimationFrame(this.measureScheduled,);
    this.destroyed = true;
  }
  /**
  Returns an effect that can be
  [added](https://codemirror.net/6/docs/ref/#state.TransactionSpec.effects) to a transaction to
  cause it to scroll the given position or range into view.
  */
  static scrollIntoView(pos, options = {},) {
    return scrollIntoView.of(
      new ScrollTarget(
        typeof pos == 'number' ? EditorSelection.cursor(pos,) : pos,
        options.y,
        options.x,
        options.yMargin,
        options.xMargin,
      ),
    );
  }
  /**
  Returns an extension that can be used to add DOM event handlers.
  The value should be an object mapping event names to handler
  functions. For any given event, such functions are ordered by
  extension precedence, and the first handler to return true will
  be assumed to have handled that event, and no other handlers or
  built-in behavior will be activated for it. These are registered
  on the [content element](https://codemirror.net/6/docs/ref/#view.EditorView.contentDOM), except
  for `scroll` handlers, which will be called any time the
  editor's [scroll element](https://codemirror.net/6/docs/ref/#view.EditorView.scrollDOM) or one of
  its parent nodes is scrolled.
  */
  static domEventHandlers(handlers2,) {
    return ViewPlugin.define(() => ({}), { eventHandlers: handlers2, },);
  }
  /**
  Create a theme extension. The first argument can be a
  [`style-mod`](https://github.com/marijnh/style-mod#documentation)
  style spec providing the styles for the theme. These will be
  prefixed with a generated class for the style.

  Because the selectors will be prefixed with a scope class, rule
  that directly match the editor's [wrapper
  element](https://codemirror.net/6/docs/ref/#view.EditorView.dom)—to which the scope class will be
  added—need to be explicitly differentiated by adding an `&` to
  the selector for that element—for example
  `&.cm-focused`.

  When `dark` is set to true, the theme will be marked as dark,
  which will cause the `&dark` rules from [base
  themes](https://codemirror.net/6/docs/ref/#view.EditorView^baseTheme) to be used (as opposed to
  `&light` when a light theme is active).
  */
  static theme(spec, options,) {
    let prefix = StyleModule.newName();
    let result = [theme.of(prefix,), styleModule.of(buildTheme(`.${prefix}`, spec,),),];
    if (options && options.dark) result.push(darkTheme.of(true,),);
    return result;
  }
  /**
  Create an extension that adds styles to the base theme. Like
  with [`theme`](https://codemirror.net/6/docs/ref/#view.EditorView^theme), use `&` to indicate the
  place of the editor wrapper element when directly targeting
  that. You can also use `&dark` or `&light` instead to only
  target editors with a dark or light theme.
  */
  static baseTheme(spec,) {
    return Prec.lowest(styleModule.of(buildTheme('.' + baseThemeID, spec, lightDarkIDs,),),);
  }
  /**
  Retrieve an editor view instance from the view's DOM
  representation.
  */
  static findFromDOM(dom,) {
    var _a2;
    let content2 = dom.querySelector('.cm-content',);
    let cView = content2 && ContentView.get(content2,) || ContentView.get(dom,);
    return ((_a2 = cView === null || cView === void 0 ? void 0 : cView.rootView) === null || _a2 === void 0 ? void 0 : _a2.view) || null;
  }
  /**
  Construct a new view. You'll want to either provide a `parent`
  option, or put `view.dom` into your document after creating a
  view, so that the user can see the editor.
  */
  constructor(config = {},) {
    this.plugins = [];
    this.pluginMap = /* @__PURE__ */ new Map();
    this.editorAttrs = {};
    this.contentAttrs = {};
    this.bidiCache = [];
    this.destroyed = false;
    this.updateState = 2;
    this.measureScheduled = -1;
    this.measureRequests = [];
    this.contentDOM = document.createElement('div',);
    this.scrollDOM = document.createElement('div',);
    this.scrollDOM.tabIndex = -1;
    this.scrollDOM.className = 'cm-scroller';
    this.scrollDOM.appendChild(this.contentDOM,);
    this.announceDOM = document.createElement('div',);
    this.announceDOM.style.cssText = 'position: fixed; top: -10000px';
    this.announceDOM.setAttribute('aria-live', 'polite',);
    this.dom = document.createElement('div',);
    this.dom.appendChild(this.announceDOM,);
    this.dom.appendChild(this.scrollDOM,);
    this._dispatch = config.dispatch || ((tr,) => this.update([tr,],));
    this.dispatch = this.dispatch.bind(this,);
    this._root = config.root || getRoot(config.parent,) || document;
    this.viewState = new ViewState(config.state || EditorState.create(config,),);
    this.plugins = this.state.facet(viewPlugin,).map((spec,) => new PluginInstance(spec,));
    for (let plugin2 of this.plugins) plugin2.update(this,);
    this.observer = new DOMObserver(this,);
    this.inputState = new InputState(this,);
    this.inputState.ensureHandlers(this, this.plugins,);
    this.docView = new DocView(this,);
    this.mountStyles();
    this.updateAttrs();
    this.updateState = 0;
    this.requestMeasure();
    if (config.parent) config.parent.appendChild(this.dom,);
  }
};
EditorView.styleModule = styleModule;
EditorView.inputHandler = inputHandler;
EditorView.focusChangeEffect = focusChangeEffect;
EditorView.perLineTextDirection = perLineTextDirection;
EditorView.exceptionSink = exceptionSink;
EditorView.updateListener = updateListener;
EditorView.editable = editable;
EditorView.mouseSelectionStyle = mouseSelectionStyle;
EditorView.dragMovesSelection = dragMovesSelection$1;
EditorView.clickAddsSelectionRange = clickAddsSelectionRange;
EditorView.decorations = decorations;
EditorView.atomicRanges = atomicRanges;
EditorView.scrollMargins = scrollMargins;
EditorView.darkTheme = darkTheme;
EditorView.contentAttributes = contentAttributes;
EditorView.editorAttributes = editorAttributes;
EditorView.lineWrapping = /* @__PURE__ */ EditorView.contentAttributes.of({ class: 'cm-lineWrapping', },);
EditorView.announce = /* @__PURE__ */ StateEffect.define();
var MaxBidiLine = 4096;
var BadMeasure = {};
var CachedOrder = class {
  static update(cache, changes,) {
    if (changes.empty) return cache;
    let result = [], lastDir = cache.length ? cache[cache.length - 1].dir : Direction.LTR;
    for (let i2 = Math.max(0, cache.length - 10,); i2 < cache.length; i2++) {
      let entry = cache[i2];
      if (entry.dir == lastDir && !changes.touchesRange(entry.from, entry.to,)) {
        result.push(new CachedOrder(changes.mapPos(entry.from, 1,), changes.mapPos(entry.to, -1,), entry.dir, entry.order,),);
      }
    }
    return result;
  }
  constructor(from, to, dir, order,) {
    this.from = from;
    this.to = to;
    this.dir = dir;
    this.order = order;
  }
};
function attrsFromFacet(view, facet, base2,) {
  for (let sources = view.state.facet(facet,), i2 = sources.length - 1; i2 >= 0; i2--) {
    let source = sources[i2], value = typeof source == 'function' ? source(view,) : source;
    if (value) combineAttrs(value, base2,);
  }
  return base2;
}
var currentPlatform = browser.mac ? 'mac' : browser.windows ? 'win' : browser.linux ? 'linux' : 'key';
function normalizeKeyName(name2, platform,) {
  const parts = name2.split(/-(?!$)/,);
  let result = parts[parts.length - 1];
  if (result == 'Space') result = ' ';
  let alt, ctrl, shift2, meta2;
  for (let i2 = 0; i2 < parts.length - 1; ++i2) {
    const mod = parts[i2];
    if (/^(cmd|meta|m)$/i.test(mod,)) meta2 = true;
    else if (/^a(lt)?$/i.test(mod,)) alt = true;
    else if (/^(c|ctrl|control)$/i.test(mod,)) ctrl = true;
    else if (/^s(hift)?$/i.test(mod,)) shift2 = true;
    else if (/^mod$/i.test(mod,)) {
      if (platform == 'mac') meta2 = true;
      else ctrl = true;
    } else throw new Error('Unrecognized modifier name: ' + mod,);
  }
  if (alt) result = 'Alt-' + result;
  if (ctrl) result = 'Ctrl-' + result;
  if (meta2) result = 'Meta-' + result;
  if (shift2) result = 'Shift-' + result;
  return result;
}
function modifiers(name2, event, shift2,) {
  if (event.altKey) name2 = 'Alt-' + name2;
  if (event.ctrlKey) name2 = 'Ctrl-' + name2;
  if (event.metaKey) name2 = 'Meta-' + name2;
  if (shift2 !== false && event.shiftKey) name2 = 'Shift-' + name2;
  return name2;
}
var handleKeyEvents = /* @__PURE__ */ Prec.default(/* @__PURE__ */ EditorView.domEventHandlers({
  keydown(event, view,) {
    return runHandlers(getKeymap(view.state,), event, view, 'editor',);
  },
},),);
var keymap = /* @__PURE__ */ Facet.define({ enables: handleKeyEvents, },);
var Keymaps = /* @__PURE__ */ new WeakMap();
function getKeymap(state,) {
  let bindings = state.facet(keymap,);
  let map = Keymaps.get(bindings,);
  if (!map) Keymaps.set(bindings, map = buildKeymap(bindings.reduce((a, b,) => a.concat(b,), [],),),);
  return map;
}
var storedPrefix = null;
var PrefixTimeout = 4e3;
function buildKeymap(bindings, platform = currentPlatform,) {
  let bound = /* @__PURE__ */ Object.create(null,);
  let isPrefix = /* @__PURE__ */ Object.create(null,);
  let checkPrefix = (name2, is,) => {
    let current = isPrefix[name2];
    if (current == null) isPrefix[name2] = is;
    else if (current != is) throw new Error('Key binding ' + name2 + ' is used both as a regular binding and as a multi-stroke prefix',);
  };
  let add = (scope, key, command, preventDefault,) => {
    var _a2, _b;
    let scopeObj = bound[scope] || (bound[scope] = /* @__PURE__ */ Object.create(null,));
    let parts = key.split(/ (?!$)/,).map((k,) => normalizeKeyName(k, platform,));
    for (let i2 = 1; i2 < parts.length; i2++) {
      let prefix = parts.slice(0, i2,).join(' ',);
      checkPrefix(prefix, true,);
      if (!scopeObj[prefix]) {
        scopeObj[prefix] = {
          preventDefault: true,
          run: [(view,) => {
            let ourObj = storedPrefix = { view, prefix, scope, };
            setTimeout(() => {
              if (storedPrefix == ourObj) storedPrefix = null;
            }, PrefixTimeout,);
            return true;
          },],
        };
      }
    }
    let full = parts.join(' ',);
    checkPrefix(full, false,);
    let binding = scopeObj[full] ||
      (scopeObj[full] = {
        preventDefault: false,
        run: ((_b = (_a2 = scopeObj._any) === null || _a2 === void 0 ? void 0 : _a2.run) === null || _b === void 0 ? void 0 : _b.slice()) ||
          [],
      });
    if (command) binding.run.push(command,);
    if (preventDefault) binding.preventDefault = true;
  };
  for (let b of bindings) {
    let scopes = b.scope ? b.scope.split(' ',) : ['editor',];
    if (b.any) {
      for (let scope of scopes) {
        let scopeObj = bound[scope] || (bound[scope] = /* @__PURE__ */ Object.create(null,));
        if (!scopeObj._any) scopeObj._any = { preventDefault: false, run: [], };
        for (let key in scopeObj) scopeObj[key].run.push(b.any,);
      }
    }
    let name2 = b[platform] || b.key;
    if (!name2) continue;
    for (let scope1 of scopes) {
      add(scope1, name2, b.run, b.preventDefault,);
      if (b.shift) add(scope1, 'Shift-' + name2, b.shift, b.preventDefault,);
    }
  }
  return bound;
}
function runHandlers(map, event, view, scope,) {
  let name2 = keyName(event,);
  let charCode = codePointAt(name2, 0,), isChar = codePointSize(charCode,) == name2.length && name2 != ' ';
  let prefix = '', fallthrough = false;
  if (storedPrefix && storedPrefix.view == view && storedPrefix.scope == scope) {
    prefix = storedPrefix.prefix + ' ';
    if (fallthrough = modifierCodes.indexOf(event.keyCode,) < 0) storedPrefix = null;
  }
  let ran = /* @__PURE__ */ new Set();
  let runFor = (binding,) => {
    if (binding) {
      for (let cmd of binding.run) {
        if (!ran.has(cmd,)) {
          ran.add(cmd,);
          if (cmd(view, event,)) return true;
        }
      }
      if (binding.preventDefault) fallthrough = true;
    }
    return false;
  };
  let scopeObj = map[scope], baseName, shiftName;
  if (scopeObj) {
    if (runFor(scopeObj[prefix + modifiers(name2, event, !isChar,)],)) return true;
    if (
      isChar && (event.altKey || event.metaKey || event.ctrlKey) && !(browser.windows && event.ctrlKey && event.altKey) &&
      (baseName = base[event.keyCode]) && baseName != name2
    ) {
      if (runFor(scopeObj[prefix + modifiers(baseName, event, true,)],)) return true;
      else if (
        event.shiftKey && (shiftName = shift[event.keyCode]) != name2 && shiftName != baseName &&
        runFor(scopeObj[prefix + modifiers(shiftName, event, false,)],)
      ) return true;
    } else if (isChar && event.shiftKey) {
      if (runFor(scopeObj[prefix + modifiers(name2, event, true,)],)) return true;
    }
    if (runFor(scopeObj._any,)) return true;
  }
  return fallthrough;
}
var CanHidePrimary = !browser.ios;
var themeSpec = {
  '.cm-line': {
    '& ::selection': { backgroundColor: 'transparent !important', },
    '&::selection': { backgroundColor: 'transparent !important', },
  },
};
if (CanHidePrimary) themeSpec['.cm-line'].caretColor = 'transparent !important';
function iterMatches(doc2, re, from, to, f,) {
  re.lastIndex = 0;
  for (let cursor = doc2.iterRange(from, to,), pos = from, m; !cursor.next().done; pos += cursor.value.length) {
    if (!cursor.lineBreak) while (m = re.exec(cursor.value,)) f(pos + m.index, m,);
  }
}
function matchRanges(view, maxLength,) {
  let visible = view.visibleRanges;
  if (visible.length == 1 && visible[0].from == view.viewport.from && visible[0].to == view.viewport.to) return visible;
  let result = [];
  for (let { from, to, } of visible) {
    from = Math.max(view.state.doc.lineAt(from,).from, from - maxLength,);
    to = Math.min(view.state.doc.lineAt(to,).to, to + maxLength,);
    if (result.length && result[result.length - 1].to >= from) result[result.length - 1].to = to;
    else result.push({ from, to, },);
  }
  return result;
}
var MatchDecorator = class {
  /**
  Compute the full set of decorations for matches in the given
  view's viewport. You'll want to call this when initializing your
  plugin.
  */
  createDeco(view,) {
    let build = new RangeSetBuilder(), add = build.add.bind(build,);
    for (let { from, to, } of matchRanges(view, this.maxLength,)) {
      iterMatches(view.state.doc, this.regexp, from, to, (from2, m,) =>
        this.addMatch(m, view, from2, add,),);
    }
    return build.finish();
  }
  /**
  Update a set of decorations for a view update. `deco` _must_ be
  the set of decorations produced by _this_ `MatchDecorator` for
  the view state before the update.
  */
  updateDeco(update, deco,) {
    let changeFrom = 1e9, changeTo = -1;
    if (update.docChanged) {
      update.changes.iterChanges((_f, _t, from, to,) => {
        if (to > update.view.viewport.from && from < update.view.viewport.to) {
          changeFrom = Math.min(from, changeFrom,);
          changeTo = Math.max(to, changeTo,);
        }
      },);
    }
    if (update.viewportChanged || changeTo - changeFrom > 1e3) return this.createDeco(update.view,);
    if (changeTo > -1) return this.updateRange(update.view, deco.map(update.changes,), changeFrom, changeTo,);
    return deco;
  }
  updateRange(view, deco, updateFrom, updateTo,) {
    for (let r of view.visibleRanges) {
      let from = Math.max(r.from, updateFrom,), to = Math.min(r.to, updateTo,);
      if (to > from) {
        let fromLine = view.state.doc.lineAt(from,), toLine = fromLine.to < to ? view.state.doc.lineAt(to,) : fromLine;
        let start = Math.max(r.from, fromLine.from,), end = Math.min(r.to, toLine.to,);
        if (this.boundary) {
          for (; from > fromLine.from; from--) {
            if (this.boundary.test(fromLine.text[from - 1 - fromLine.from],)) {
              start = from;
              break;
            }
          }
          for (; to < toLine.to; to++) {
            if (this.boundary.test(toLine.text[to - toLine.from],)) {
              end = to;
              break;
            }
          }
        }
        let ranges = [], m;
        let add = (from2, to2, deco2,) => ranges.push(deco2.range(from2, to2,),);
        if (fromLine == toLine) {
          this.regexp.lastIndex = start - fromLine.from;
          while ((m = this.regexp.exec(fromLine.text,)) && m.index < end - fromLine.from) {
            this.addMatch(m, view, m.index + fromLine.from, add,);
          }
        } else {
          iterMatches(view.state.doc, this.regexp, start, end, (from2, m2,) => this.addMatch(m2, view, from2, add,),);
        }
        deco = deco.update({ filterFrom: start, filterTo: end, filter: (from2, to2,) => from2 < start || to2 > end, add: ranges, },);
      }
    }
    return deco;
  }
  /**
  Create a decorator.
  */
  constructor(config,) {
    const { regexp, decoration, decorate, boundary, maxLength = 1e3, } = config;
    if (!regexp.global) throw new RangeError('The regular expression given to MatchDecorator should have its \'g\' flag set',);
    this.regexp = regexp;
    if (decorate) {
      this.addMatch = (match, view, from, add,) => decorate(add, from, from + match[0].length, match, view,);
    } else if (typeof decoration == 'function') {
      this.addMatch = (match, view, from, add,) => {
        let deco = decoration(match, view, from,);
        if (deco) add(from, from + match[0].length, deco,);
      };
    } else if (decoration) {
      this.addMatch = (match, _view, from, add,) => add(from, from + match[0].length, decoration,);
    } else {
      throw new RangeError('Either \'decorate\' or \'decoration\' should be provided to MatchDecorator',);
    }
    this.boundary = boundary;
    this.maxLength = maxLength;
  }
};
var UnicodeRegexpSupport = /x/.unicode != null ? 'gu' : 'g';
var Specials = /* @__PURE__ */ new RegExp(
  '[\0-\b\n-\x7F-\x9F\xAD\u061C\u200B\u200E\u200F\u2028\u2029\u202D\u202E\u2066\u2067\u2069\uFEFF\uFFF9-\uFFFC]',
  UnicodeRegexpSupport,
);
var Names = {
  0: 'null',
  7: 'bell',
  8: 'backspace',
  10: 'newline',
  11: 'vertical tab',
  13: 'carriage return',
  27: 'escape',
  8203: 'zero width space',
  8204: 'zero width non-joiner',
  8205: 'zero width joiner',
  8206: 'left-to-right mark',
  8207: 'right-to-left mark',
  8232: 'line separator',
  8237: 'left-to-right override',
  8238: 'right-to-left override',
  8294: 'left-to-right isolate',
  8295: 'right-to-left isolate',
  8297: 'pop directional isolate',
  8233: 'paragraph separator',
  65279: 'zero width no-break space',
  65532: 'object replacement',
};
var _supportsTabSize = null;
function supportsTabSize() {
  var _a2;
  if (_supportsTabSize == null && typeof document != 'undefined' && document.body) {
    let styles = document.body.style;
    _supportsTabSize = ((_a2 = styles.tabSize) !== null && _a2 !== void 0 ? _a2 : styles.MozTabSize) != null;
  }
  return _supportsTabSize || false;
}
var specialCharConfig = /* @__PURE__ */ Facet.define({
  combine(configs,) {
    let config = combineConfig(configs, { render: null, specialChars: Specials, addSpecialChars: null, },);
    if (config.replaceTabs = !supportsTabSize()) {
      config.specialChars = new RegExp('	|' + config.specialChars.source, UnicodeRegexpSupport,);
    }
    if (config.addSpecialChars) {
      config.specialChars = new RegExp(config.specialChars.source + '|' + config.addSpecialChars.source, UnicodeRegexpSupport,);
    }
    return config;
  },
},);
function highlightSpecialChars(config = {},) {
  return [specialCharConfig.of(config,), specialCharPlugin(),];
}
var _plugin = null;
function specialCharPlugin() {
  return _plugin || (_plugin = ViewPlugin.fromClass(
    class {
      makeDecorator(conf,) {
        return new MatchDecorator({
          regexp: conf.specialChars,
          decoration: (m, view, pos,) => {
            let { doc: doc2, } = view.state;
            let code2 = codePointAt(m[0], 0,);
            if (code2 == 9) {
              let line = doc2.lineAt(pos,);
              let size = view.state.tabSize, col = countColumn(line.text, size, pos - line.from,);
              return Decoration.replace({ widget: new TabWidget((size - col % size) * this.view.defaultCharacterWidth,), },);
            }
            return this.decorationCache[code2] ||
              (this.decorationCache[code2] = Decoration.replace({ widget: new SpecialCharWidget(conf, code2,), },));
          },
          boundary: conf.replaceTabs ? void 0 : /[^]/,
        },);
      }
      update(update,) {
        let conf = update.state.facet(specialCharConfig,);
        if (update.startState.facet(specialCharConfig,) != conf) {
          this.decorator = this.makeDecorator(conf,);
          this.decorations = this.decorator.createDeco(update.view,);
        } else {
          this.decorations = this.decorator.updateDeco(update, this.decorations,);
        }
      }
      constructor(view,) {
        this.view = view;
        this.decorations = Decoration.none;
        this.decorationCache = /* @__PURE__ */ Object.create(null,);
        this.decorator = this.makeDecorator(view.state.facet(specialCharConfig,),);
        this.decorations = this.decorator.createDeco(view,);
      }
    },
    { decorations: (v,) => v.decorations, },
  ));
}
var DefaultPlaceholder = '\u2022';
function placeholder$1(code2,) {
  if (code2 >= 32) return DefaultPlaceholder;
  if (code2 == 10) return '\u2424';
  return String.fromCharCode(9216 + code2,);
}
var SpecialCharWidget = class extends WidgetType {
  eq(other,) {
    return other.code == this.code;
  }
  toDOM(view,) {
    let ph = placeholder$1(this.code,);
    let desc = view.state.phrase('Control character',) + ' ' + (Names[this.code] || '0x' + this.code.toString(16,));
    let custom = this.options.render && this.options.render(this.code, desc, ph,);
    if (custom) return custom;
    let span = document.createElement('span',);
    span.textContent = ph;
    span.title = desc;
    span.setAttribute('aria-label', desc,);
    span.className = 'cm-specialChar';
    return span;
  }
  ignoreEvent() {
    return false;
  }
  constructor(options, code2,) {
    super();
    this.options = options;
    this.code = code2;
  }
};
var TabWidget = class extends WidgetType {
  eq(other,) {
    return other.width == this.width;
  }
  toDOM() {
    let span = document.createElement('span',);
    span.textContent = '	';
    span.className = 'cm-tab';
    span.style.width = this.width + 'px';
    return span;
  }
  ignoreEvent() {
    return false;
  }
  constructor(width,) {
    super();
    this.width = width;
  }
};
function highlightActiveLine() {
  return activeLineHighlighter;
}
var lineDeco = /* @__PURE__ */ Decoration.line({ class: 'cm-activeLine', },);
var activeLineHighlighter = /* @__PURE__ */ ViewPlugin.fromClass(
  class {
    update(update,) {
      if (update.docChanged || update.selectionSet) this.decorations = this.getDeco(update.view,);
    }
    getDeco(view,) {
      let lastLineStart = -1, deco = [];
      for (let r of view.state.selection.ranges) {
        let line = view.lineBlockAt(r.head,);
        if (line.from > lastLineStart) {
          deco.push(lineDeco.range(line.from,),);
          lastLineStart = line.from;
        }
      }
      return Decoration.set(deco,);
    }
    constructor(view,) {
      this.decorations = this.getDeco(view,);
    }
  },
  { decorations: (v,) => v.decorations, },
);
var baseTheme = /* @__PURE__ */ EditorView.baseTheme({
  '.cm-tooltip': { zIndex: 100, boxSizing: 'border-box', },
  '&light .cm-tooltip': { border: '1px solid #bbb', backgroundColor: '#f5f5f5', },
  '&light .cm-tooltip-section:not(:first-child)': { borderTop: '1px solid #bbb', },
  '&dark .cm-tooltip': { backgroundColor: '#333338', color: 'white', },
  '.cm-tooltip-arrow': {
    height: `${7}px`,
    width: `${7 * 2}px`,
    position: 'absolute',
    zIndex: -1,
    overflow: 'hidden',
    '&:before, &:after': {
      content: '\'\'',
      position: 'absolute',
      width: 0,
      height: 0,
      borderLeft: `${7}px solid transparent`,
      borderRight: `${7}px solid transparent`,
    },
    '.cm-tooltip-above &': {
      bottom: `-${7}px`,
      '&:before': { borderTop: `${7}px solid #bbb`, },
      '&:after': { borderTop: `${7}px solid #f5f5f5`, bottom: '1px', },
    },
    '.cm-tooltip-below &': {
      top: `-${7}px`,
      '&:before': { borderBottom: `${7}px solid #bbb`, },
      '&:after': { borderBottom: `${7}px solid #f5f5f5`, top: '1px', },
    },
  },
  '&dark .cm-tooltip .cm-tooltip-arrow': {
    '&:before': { borderTopColor: '#333338', borderBottomColor: '#333338', },
    '&:after': { borderTopColor: 'transparent', borderBottomColor: 'transparent', },
  },
},);
var GutterMarker = class extends RangeValue {
  /**
  @internal
  */
  compare(other,) {
    return this == other || this.constructor == other.constructor && this.eq(other,);
  }
  /**
  Compare this marker to another marker of the same type.
  */
  eq(other,) {
    return false;
  }
  /**
  Called if the marker has a `toDOM` method and its representation
  was removed from a gutter.
  */
  destroy(dom,) {
  }
};
GutterMarker.prototype.elementClass = '';
GutterMarker.prototype.toDOM = void 0;
GutterMarker.prototype.mapMode = MapMode.TrackBefore;
GutterMarker.prototype.startSide = GutterMarker.prototype.endSide = -1;
GutterMarker.prototype.point = true;
var gutterLineClass = /* @__PURE__ */ Facet.define();
var activeGutters = /* @__PURE__ */ Facet.define();
var unfixGutters = /* @__PURE__ */ Facet.define({ combine: (values,) => values.some((x,) => x), },);
function gutters(config,) {
  let result = [gutterView,];
  if (config && config.fixed === false) result.push(unfixGutters.of(true,),);
  return result;
}
var gutterView = /* @__PURE__ */ ViewPlugin.fromClass(
  class {
    update(update,) {
      if (this.updateGutters(update,)) {
        let vpA = this.prevViewport, vpB = update.view.viewport;
        let vpOverlap = Math.min(vpA.to, vpB.to,) - Math.max(vpA.from, vpB.from,);
        this.syncGutters(vpOverlap < (vpB.to - vpB.from) * 0.8,);
      }
      if (update.geometryChanged) this.dom.style.minHeight = this.view.contentHeight + 'px';
      if (this.view.state.facet(unfixGutters,) != !this.fixed) {
        this.fixed = !this.fixed;
        this.dom.style.position = this.fixed ? 'sticky' : '';
      }
      this.prevViewport = update.view.viewport;
    }
    syncGutters(detach,) {
      let after = this.dom.nextSibling;
      if (detach) this.dom.remove();
      let lineClasses = RangeSet.iter(this.view.state.facet(gutterLineClass,), this.view.viewport.from,);
      let classSet = [];
      let contexts = this.gutters.map((gutter2,) => new UpdateContext(gutter2, this.view.viewport, -this.view.documentPadding.top,));
      for (let line of this.view.viewportLineBlocks) {
        if (classSet.length) classSet = [];
        if (Array.isArray(line.type,)) {
          let first = true;
          for (let b of line.type) {
            if (b.type == BlockType.Text && first) {
              advanceCursor(lineClasses, classSet, b.from,);
              for (let cx of contexts) cx.line(this.view, b, classSet,);
              first = false;
            } else if (b.widget) {
              for (let cx1 of contexts) cx1.widget(this.view, b,);
            }
          }
        } else if (line.type == BlockType.Text) {
          advanceCursor(lineClasses, classSet, line.from,);
          for (let cx2 of contexts) cx2.line(this.view, line, classSet,);
        }
      }
      for (let cx3 of contexts) cx3.finish();
      if (detach) this.view.scrollDOM.insertBefore(this.dom, after,);
    }
    updateGutters(update,) {
      let prev = update.startState.facet(activeGutters,), cur = update.state.facet(activeGutters,);
      let change = update.docChanged || update.heightChanged || update.viewportChanged ||
        !RangeSet.eq(
          update.startState.facet(gutterLineClass,),
          update.state.facet(gutterLineClass,),
          update.view.viewport.from,
          update.view.viewport.to,
        );
      if (prev == cur) {
        for (let gutter2 of this.gutters) if (gutter2.update(update,)) change = true;
      } else {
        change = true;
        let gutters2 = [];
        for (let conf of cur) {
          let known = prev.indexOf(conf,);
          if (known < 0) {
            gutters2.push(new SingleGutterView(this.view, conf,),);
          } else {
            this.gutters[known].update(update,);
            gutters2.push(this.gutters[known],);
          }
        }
        for (let g of this.gutters) {
          g.dom.remove();
          if (gutters2.indexOf(g,) < 0) g.destroy();
        }
        for (let g1 of gutters2) this.dom.appendChild(g1.dom,);
        this.gutters = gutters2;
      }
      return change;
    }
    destroy() {
      for (let view of this.gutters) view.destroy();
      this.dom.remove();
    }
    constructor(view,) {
      this.view = view;
      this.prevViewport = view.viewport;
      this.dom = document.createElement('div',);
      this.dom.className = 'cm-gutters';
      this.dom.setAttribute('aria-hidden', 'true',);
      this.dom.style.minHeight = this.view.contentHeight + 'px';
      this.gutters = view.state.facet(activeGutters,).map((conf,) => new SingleGutterView(view, conf,));
      for (let gutter2 of this.gutters) this.dom.appendChild(gutter2.dom,);
      this.fixed = !view.state.facet(unfixGutters,);
      if (this.fixed) {
        this.dom.style.position = 'sticky';
      }
      this.syncGutters(false,);
      view.scrollDOM.insertBefore(this.dom, view.contentDOM,);
    }
  },
  {
    provide: (plugin2,) =>
      EditorView.scrollMargins.of((view,) => {
        let value = view.plugin(plugin2,);
        if (!value || value.gutters.length == 0 || !value.fixed) return null;
        return view.textDirection == Direction.LTR ? { left: value.dom.offsetWidth, } : { right: value.dom.offsetWidth, };
      },),
  },
);
function asArray2(val,) {
  return Array.isArray(val,) ? val : [val,];
}
function advanceCursor(cursor, collect, pos,) {
  while (cursor.value && cursor.from <= pos) {
    if (cursor.from == pos) collect.push(cursor.value,);
    cursor.next();
  }
}
var UpdateContext = class {
  addElement(view, block, markers,) {
    let { gutter: gutter2, } = this, above = block.top - this.height;
    if (this.i == gutter2.elements.length) {
      let newElt = new GutterElement(view, block.height, above, markers,);
      gutter2.elements.push(newElt,);
      gutter2.dom.appendChild(newElt.dom,);
    } else {
      gutter2.elements[this.i].update(view, block.height, above, markers,);
    }
    this.height = block.bottom;
    this.i++;
  }
  line(view, line, extraMarkers,) {
    let localMarkers = [];
    advanceCursor(this.cursor, localMarkers, line.from,);
    if (extraMarkers.length) localMarkers = localMarkers.concat(extraMarkers,);
    let forLine = this.gutter.config.lineMarker(view, line, localMarkers,);
    if (forLine) localMarkers.unshift(forLine,);
    let gutter2 = this.gutter;
    if (localMarkers.length == 0 && !gutter2.config.renderEmptyElements) return;
    this.addElement(view, line, localMarkers,);
  }
  widget(view, block,) {
    let marker = this.gutter.config.widgetMarker(view, block.widget, block,);
    if (marker) this.addElement(view, block, [marker,],);
  }
  finish() {
    let gutter2 = this.gutter;
    while (gutter2.elements.length > this.i) {
      let last = gutter2.elements.pop();
      gutter2.dom.removeChild(last.dom,);
      last.destroy();
    }
  }
  constructor(gutter2, viewport, height,) {
    this.gutter = gutter2;
    this.height = height;
    this.i = 0;
    this.cursor = RangeSet.iter(gutter2.markers, viewport.from,);
  }
};
var SingleGutterView = class {
  update(update,) {
    let prevMarkers = this.markers;
    this.markers = asArray2(this.config.markers(update.view,),);
    if (this.spacer && this.config.updateSpacer) {
      let updated = this.config.updateSpacer(this.spacer.markers[0], update,);
      if (updated != this.spacer.markers[0]) this.spacer.update(update.view, 0, 0, [updated,],);
    }
    let vp = update.view.viewport;
    return !RangeSet.eq(this.markers, prevMarkers, vp.from, vp.to,) ||
      (this.config.lineMarkerChange ? this.config.lineMarkerChange(update,) : false);
  }
  destroy() {
    for (let elt of this.elements) elt.destroy();
  }
  constructor(view, config,) {
    this.view = view;
    this.config = config;
    this.elements = [];
    this.spacer = null;
    this.dom = document.createElement('div',);
    this.dom.className = 'cm-gutter' + (this.config.class ? ' ' + this.config.class : '');
    for (let prop in config.domEventHandlers) {
      this.dom.addEventListener(prop, (event,) => {
        let target = event.target, y;
        if (target != this.dom && this.dom.contains(target,)) {
          while (target.parentNode != this.dom) target = target.parentNode;
          let rect = target.getBoundingClientRect();
          y = (rect.top + rect.bottom) / 2;
        } else {
          y = event.clientY;
        }
        let line = view.lineBlockAtHeight(y - view.documentTop,);
        if (config.domEventHandlers[prop](view, line, event,)) event.preventDefault();
      },);
    }
    this.markers = asArray2(config.markers(view,),);
    if (config.initialSpacer) {
      this.spacer = new GutterElement(view, 0, 0, [config.initialSpacer(view,),],);
      this.dom.appendChild(this.spacer.dom,);
      this.spacer.dom.style.cssText += 'visibility: hidden; pointer-events: none';
    }
  }
};
var GutterElement = class {
  update(view, height, above, markers,) {
    if (this.height != height) this.dom.style.height = (this.height = height) + 'px';
    if (this.above != above) this.dom.style.marginTop = (this.above = above) ? above + 'px' : '';
    if (!sameMarkers(this.markers, markers,)) this.setMarkers(view, markers,);
  }
  setMarkers(view, markers,) {
    let cls = 'cm-gutterElement', domPos = this.dom.firstChild;
    for (let iNew = 0, iOld = 0;;) {
      let skipTo = iOld, marker = iNew < markers.length ? markers[iNew++] : null, matched = false;
      if (marker) {
        let c = marker.elementClass;
        if (c) cls += ' ' + c;
        for (let i2 = iOld; i2 < this.markers.length; i2++) {
          if (this.markers[i2].compare(marker,)) {
            skipTo = i2;
            matched = true;
            break;
          }
        }
      } else {
        skipTo = this.markers.length;
      }
      while (iOld < skipTo) {
        let next = this.markers[iOld++];
        if (next.toDOM) {
          next.destroy(domPos,);
          let after = domPos.nextSibling;
          domPos.remove();
          domPos = after;
        }
      }
      if (!marker) break;
      if (marker.toDOM) {
        if (matched) domPos = domPos.nextSibling;
        else this.dom.insertBefore(marker.toDOM(view,), domPos,);
      }
      if (matched) iOld++;
    }
    this.dom.className = cls;
    this.markers = markers;
  }
  destroy() {
    this.setMarkers(null, [],);
  }
  constructor(view, height, above, markers,) {
    this.height = -1;
    this.above = 0;
    this.markers = [];
    this.dom = document.createElement('div',);
    this.dom.className = 'cm-gutterElement';
    this.update(view, height, above, markers,);
  }
};
function sameMarkers(a, b,) {
  if (a.length != b.length) return false;
  for (let i2 = 0; i2 < a.length; i2++) if (!a[i2].compare(b[i2],)) return false;
  return true;
}
var lineNumberMarkers = /* @__PURE__ */ Facet.define();
var lineNumberConfig = /* @__PURE__ */ Facet.define({
  combine(values,) {
    return combineConfig(values, { formatNumber: String, domEventHandlers: {}, }, {
      domEventHandlers(a, b,) {
        let result = Object.assign({}, a,);
        for (let event in b) {
          let exists = result[event], add = b[event];
          result[event] = exists ? (view, line, event2,) => exists(view, line, event2,) || add(view, line, event2,) : add;
        }
        return result;
      },
    },);
  },
},);
var NumberMarker = class extends GutterMarker {
  eq(other,) {
    return this.number == other.number;
  }
  toDOM() {
    return document.createTextNode(this.number,);
  }
  constructor(number2,) {
    super();
    this.number = number2;
  }
};
function formatNumber(view, number2,) {
  return view.state.facet(lineNumberConfig,).formatNumber(number2, view.state,);
}
var lineNumberGutter = /* @__PURE__ */ activeGutters.compute(
  [lineNumberConfig,],
  (state,) => ({
    class: 'cm-lineNumbers',
    renderEmptyElements: false,
    markers(view,) {
      return view.state.facet(lineNumberMarkers,);
    },
    lineMarker(view, line, others,) {
      if (others.some((m,) => m.toDOM)) return null;
      return new NumberMarker(formatNumber(view, view.state.doc.lineAt(line.from,).number,),);
    },
    widgetMarker: () => null,
    lineMarkerChange: (update,) => update.startState.facet(lineNumberConfig,) != update.state.facet(lineNumberConfig,),
    initialSpacer(view,) {
      return new NumberMarker(formatNumber(view, maxLineNumber(view.state.doc.lines,),),);
    },
    updateSpacer(spacer, update,) {
      let max = formatNumber(update.view, maxLineNumber(update.view.state.doc.lines,),);
      return max == spacer.number ? spacer : new NumberMarker(max,);
    },
    domEventHandlers: state.facet(lineNumberConfig,).domEventHandlers,
  }),
);
function lineNumbers(config = {},) {
  return [lineNumberConfig.of(config,), gutters(), lineNumberGutter,];
}
function maxLineNumber(lines,) {
  let last = 9;
  while (last < lines) last = last * 10 + 9;
  return last;
}

// https :https://framerusercontent.com/modules/CO9ruPlVDRTCLVInyri7/dIK6ma4pzS0qSHWtJ4s1/lezer_common.js
var DefaultBufferLength = 1024;
var nextPropID = 0;
var Range2 = class {
  constructor(from, to,) {
    this.from = from;
    this.to = to;
  }
};
var NodeProp = class {
  /// This is meant to be used with
  /// [`NodeSet.extend`](#common.NodeSet.extend) or
  /// [`LRParser.configure`](#lr.ParserConfig.props) to compute
  /// prop values for each node type in the set. Takes a [match
  /// object](#common.NodeType^match) or function that returns undefined
  /// if the node type doesn't get this prop, and the prop's value if
  /// it does.
  add(match,) {
    if (this.perNode) throw new RangeError('Can\'t add per-node props to node types',);
    if (typeof match != 'function') match = NodeType.match(match,);
    return (type,) => {
      let result = match(type,);
      return result === void 0 ? null : [this, result,];
    };
  }
  /// Create a new node prop type.
  constructor(config = {},) {
    this.id = nextPropID++;
    this.perNode = !!config.perNode;
    this.deserialize = config.deserialize || (() => {
      throw new Error('This node type doesn\'t define a deserialize function',);
    });
  }
};
NodeProp.closedBy = new NodeProp({ deserialize: (str,) => str.split(' ',), },);
NodeProp.openedBy = new NodeProp({ deserialize: (str,) => str.split(' ',), },);
NodeProp.group = new NodeProp({ deserialize: (str,) => str.split(' ',), },);
NodeProp.contextHash = new NodeProp({ perNode: true, },);
NodeProp.lookAhead = new NodeProp({ perNode: true, },);
NodeProp.mounted = new NodeProp({ perNode: true, },);
var MountedTree = class {
  constructor(tree, overlay, parser,) {
    this.tree = tree;
    this.overlay = overlay;
    this.parser = parser;
  }
};
var noProps = /* @__PURE__ */ Object.create(null,);
var NodeType = class {
  /// Define a node type.
  static define(spec,) {
    let props = spec.props && spec.props.length ? /* @__PURE__ */ Object.create(null,) : noProps;
    let flags = (spec.top ? 1 : 0) | (spec.skipped ? 2 : 0) | (spec.error ? 4 : 0) | (spec.name == null ? 8 : 0);
    let type = new NodeType(spec.name || '', props, spec.id, flags,);
    if (spec.props) {
      for (let src of spec.props) {
        if (!Array.isArray(src,)) src = src(type,);
        if (src) {
          if (src[0].perNode) throw new RangeError('Can\'t store a per-node prop on a node type',);
          props[src[0].id] = src[1];
        }
      }
    }
    return type;
  }
  /// Retrieves a node prop for this type. Will return `undefined` if
  /// the prop isn't present on this node.
  prop(prop,) {
    return this.props[prop.id];
  }
  /// True when this is the top node of a grammar.
  get isTop() {
    return (this.flags & 1) > 0;
  }
  /// True when this node is produced by a skip rule.
  get isSkipped() {
    return (this.flags & 2) > 0;
  }
  /// Indicates whether this is an error node.
  get isError() {
    return (this.flags & 4) > 0;
  }
  /// When true, this node type doesn't correspond to a user-declared
  /// named node, for example because it is used to cache repetition.
  get isAnonymous() {
    return (this.flags & 8) > 0;
  }
  /// Returns true when this node's name or one of its
  /// [groups](#common.NodeProp^group) matches the given string.
  is(name2,) {
    if (typeof name2 == 'string') {
      if (this.name == name2) return true;
      let group = this.prop(NodeProp.group,);
      return group ? group.indexOf(name2,) > -1 : false;
    }
    return this.id == name2;
  }
  /// Create a function from node types to arbitrary values by
  /// specifying an object whose property names are node or
  /// [group](#common.NodeProp^group) names. Often useful with
  /// [`NodeProp.add`](#common.NodeProp.add). You can put multiple
  /// names, separated by spaces, in a single property name to map
  /// multiple node names to a single value.
  static match(map,) {
    let direct = /* @__PURE__ */ Object.create(null,);
    for (let prop in map) for (let name2 of prop.split(' ',)) direct[name2] = map[prop];
    return (node,) => {
      for (let groups = node.prop(NodeProp.group,), i2 = -1; i2 < (groups ? groups.length : 0); i2++) {
        let found = direct[i2 < 0 ? node.name : groups[i2]];
        if (found) return found;
      }
    };
  }
  /// @internal
  constructor(name2, props, id2, flags = 0,) {
    this.name = name2;
    this.props = props;
    this.id = id2;
    this.flags = flags;
  }
};
NodeType.none = new NodeType('', /* @__PURE__ */ Object.create(null,), 0, 8,);
var NodeSet = class {
  /// Create a copy of this set with some node properties added. The
  /// arguments to this method can be created with
  /// [`NodeProp.add`](#common.NodeProp.add).
  extend(...props) {
    let newTypes = [];
    for (let type of this.types) {
      let newProps = null;
      for (let source of props) {
        let add = source(type,);
        if (add) {
          if (!newProps) newProps = Object.assign({}, type.props,);
          newProps[add[0].id] = add[1];
        }
      }
      newTypes.push(newProps ? new NodeType(type.name, newProps, type.id, type.flags,) : type,);
    }
    return new NodeSet(newTypes,);
  }
  /// Create a set with the given types. The `id` property of each
  /// type should correspond to its position within the array.
  constructor(types2,) {
    this.types = types2;
    for (let i2 = 0; i2 < types2.length; i2++) {
      if (types2[i2].id != i2) throw new RangeError('Node type ids should correspond to array positions when creating a node set',);
    }
  }
};
var CachedNode = /* @__PURE__ */ new WeakMap();
var CachedInnerNode = /* @__PURE__ */ new WeakMap();
var IterMode;
(function (IterMode2,) {
  IterMode2[IterMode2['ExcludeBuffers'] = 1] = 'ExcludeBuffers';
  IterMode2[IterMode2['IncludeAnonymous'] = 2] = 'IncludeAnonymous';
  IterMode2[IterMode2['IgnoreMounts'] = 4] = 'IgnoreMounts';
  IterMode2[IterMode2['IgnoreOverlays'] = 8] = 'IgnoreOverlays';
})(IterMode || (IterMode = {}),);
var Tree = class {
  /// @internal
  toString() {
    let mounted = this.prop(NodeProp.mounted,);
    if (mounted && !mounted.overlay) return mounted.tree.toString();
    let children = '';
    for (let ch of this.children) {
      let str = ch.toString();
      if (str) {
        if (children) children += ',';
        children += str;
      }
    }
    return !this.type.name
      ? children
      : (/\W/.test(this.type.name,) && !this.type.isError ? JSON.stringify(this.type.name,) : this.type.name) +
        (children.length ? '(' + children + ')' : '');
  }
  /// Get a [tree cursor](#common.TreeCursor) positioned at the top of
  /// the tree. Mode can be used to [control](#common.IterMode) which
  /// nodes the cursor visits.
  cursor(mode = 0,) {
    return new TreeCursor(this.topNode, mode,);
  }
  /// Get a [tree cursor](#common.TreeCursor) pointing into this tree
  /// at the given position and side (see
  /// [`moveTo`](#common.TreeCursor.moveTo).
  cursorAt(pos, side = 0, mode = 0,) {
    let scope = CachedNode.get(this,) || this.topNode;
    let cursor = new TreeCursor(scope,);
    cursor.moveTo(pos, side,);
    CachedNode.set(this, cursor._tree,);
    return cursor;
  }
  /// Get a [syntax node](#common.SyntaxNode) object for the top of the
  /// tree.
  get topNode() {
    return new TreeNode(this, 0, 0, null,);
  }
  /// Get the [syntax node](#common.SyntaxNode) at the given position.
  /// If `side` is -1, this will move into nodes that end at the
  /// position. If 1, it'll move into nodes that start at the
  /// position. With 0, it'll only enter nodes that cover the position
  /// from both sides.
  ///
  /// Note that this will not enter
  /// [overlays](#common.MountedTree.overlay), and you often want
  /// [`resolveInner`](#common.Tree.resolveInner) instead.
  resolve(pos, side = 0,) {
    let node = resolveNode(CachedNode.get(this,) || this.topNode, pos, side, false,);
    CachedNode.set(this, node,);
    return node;
  }
  /// Like [`resolve`](#common.Tree.resolve), but will enter
  /// [overlaid](#common.MountedTree.overlay) nodes, producing a syntax node
  /// pointing into the innermost overlaid tree at the given position
  /// (with parent links going through all parent structure, including
  /// the host trees).
  resolveInner(pos, side = 0,) {
    let node = resolveNode(CachedInnerNode.get(this,) || this.topNode, pos, side, true,);
    CachedInnerNode.set(this, node,);
    return node;
  }
  /// Iterate over the tree and its children, calling `enter` for any
  /// node that touches the `from`/`to` region (if given) before
  /// running over such a node's children, and `leave` (if given) when
  /// leaving the node. When `enter` returns `false`, that node will
  /// not have its children iterated over (or `leave` called).
  iterate(spec,) {
    let { enter, leave, from = 0, to = this.length, } = spec;
    let mode = spec.mode || 0, anon = (mode & IterMode.IncludeAnonymous) > 0;
    for (let c = this.cursor(mode | IterMode.IncludeAnonymous,);;) {
      let entered = false;
      if (c.from <= to && c.to >= from && (!anon && c.type.isAnonymous || enter(c,) !== false)) {
        if (c.firstChild()) continue;
        entered = true;
      }
      for (;;) {
        if (entered && leave && (anon || !c.type.isAnonymous)) leave(c,);
        if (c.nextSibling()) break;
        if (!c.parent()) return;
        entered = true;
      }
    }
  }
  /// Get the value of the given [node prop](#common.NodeProp) for this
  /// node. Works with both per-node and per-type props.
  prop(prop,) {
    return !prop.perNode ? this.type.prop(prop,) : this.props ? this.props[prop.id] : void 0;
  }
  /// Returns the node's [per-node props](#common.NodeProp.perNode) in a
  /// format that can be passed to the [`Tree`](#common.Tree)
  /// constructor.
  get propValues() {
    let result = [];
    if (this.props) for (let id2 in this.props) result.push([+id2, this.props[id2],],);
    return result;
  }
  /// Balance the direct children of this tree, producing a copy of
  /// which may have children grouped into subtrees with type
  /// [`NodeType.none`](#common.NodeType^none).
  balance(config = {},) {
    return this.children.length <= 8
      ? this
      : balanceRange(
        NodeType.none,
        this.children,
        this.positions,
        0,
        this.children.length,
        0,
        this.length,
        (children, positions, length,) => new Tree(this.type, children, positions, length, this.propValues,),
        config.makeTree || ((children, positions, length,) => new Tree(NodeType.none, children, positions, length,)),
      );
  }
  /// Build a tree from a postfix-ordered buffer of node information,
  /// or a cursor over such a buffer.
  static build(data,) {
    return buildTree(data,);
  }
  /// Construct a new tree. See also [`Tree.build`](#common.Tree^build).
  constructor(type, children, positions, length, props,) {
    this.type = type;
    this.children = children;
    this.positions = positions;
    this.length = length;
    this.props = null;
    if (props && props.length) {
      this.props = /* @__PURE__ */ Object.create(null,);
      for (let [prop, value,] of props) this.props[typeof prop == 'number' ? prop : prop.id] = value;
    }
  }
};
Tree.empty = new Tree(NodeType.none, [], [], 0,);
var FlatBufferCursor = class {
  get id() {
    return this.buffer[this.index - 4];
  }
  get start() {
    return this.buffer[this.index - 3];
  }
  get end() {
    return this.buffer[this.index - 2];
  }
  get size() {
    return this.buffer[this.index - 1];
  }
  get pos() {
    return this.index;
  }
  next() {
    this.index -= 4;
  }
  fork() {
    return new FlatBufferCursor(this.buffer, this.index,);
  }
  constructor(buffer, index,) {
    this.buffer = buffer;
    this.index = index;
  }
};
var TreeBuffer = class {
  /// @internal
  get type() {
    return NodeType.none;
  }
  /// @internal
  toString() {
    let result = [];
    for (let index = 0; index < this.buffer.length;) {
      result.push(this.childString(index,),);
      index = this.buffer[index + 3];
    }
    return result.join(',',);
  }
  /// @internal
  childString(index,) {
    let id2 = this.buffer[index], endIndex = this.buffer[index + 3];
    let type = this.set.types[id2], result = type.name;
    if (/\W/.test(result,) && !type.isError) result = JSON.stringify(result,);
    index += 4;
    if (endIndex == index) return result;
    let children = [];
    while (index < endIndex) {
      children.push(this.childString(index,),);
      index = this.buffer[index + 3];
    }
    return result + '(' + children.join(',',) + ')';
  }
  /// @internal
  findChild(startIndex, endIndex, dir, pos, side,) {
    let { buffer, } = this, pick = -1;
    for (let i2 = startIndex; i2 != endIndex; i2 = buffer[i2 + 3]) {
      if (checkSide(side, pos, buffer[i2 + 1], buffer[i2 + 2],)) {
        pick = i2;
        if (dir > 0) break;
      }
    }
    return pick;
  }
  /// @internal
  slice(startI, endI, from,) {
    let b = this.buffer;
    let copy = new Uint16Array(endI - startI,), len = 0;
    for (let i2 = startI, j = 0; i2 < endI;) {
      copy[j++] = b[i2++];
      copy[j++] = b[i2++] - from;
      let to = copy[j++] = b[i2++] - from;
      copy[j++] = b[i2++] - startI;
      len = Math.max(len, to,);
    }
    return new TreeBuffer(copy, len, this.set,);
  }
  /// Create a tree buffer.
  constructor(buffer, length, set,) {
    this.buffer = buffer;
    this.length = length;
    this.set = set;
  }
};
function checkSide(side, pos, from, to,) {
  switch (side) {
    case -2:
      return from < pos;
    case -1:
      return to >= pos && from < pos;
    case 0:
      return from < pos && to > pos;
    case 1:
      return from <= pos && to > pos;
    case 2:
      return to > pos;
    case 4:
      return true;
  }
}
function enterUnfinishedNodesBefore(node, pos,) {
  let scan = node.childBefore(pos,);
  while (scan) {
    let last = scan.lastChild;
    if (!last || last.to != scan.to) break;
    if (last.type.isError && last.from == last.to) {
      node = scan;
      scan = last.prevSibling;
    } else {
      scan = last;
    }
  }
  return node;
}
function resolveNode(node, pos, side, overlays,) {
  var _a2;
  while (node.from == node.to || (side < 1 ? node.from >= pos : node.from > pos) || (side > -1 ? node.to <= pos : node.to < pos)) {
    let parent = !overlays && node instanceof TreeNode && node.index < 0 ? null : node.parent;
    if (!parent) return node;
    node = parent;
  }
  let mode = overlays ? 0 : IterMode.IgnoreOverlays;
  if (overlays) {
    for (let scan = node, parent1 = scan.parent; parent1; scan = parent1, parent1 = scan.parent) {
      if (
        scan instanceof TreeNode && scan.index < 0 &&
        ((_a2 = parent1.enter(pos, side, mode,)) === null || _a2 === void 0 ? void 0 : _a2.from) != scan.from
      ) node = parent1;
    }
  }
  for (;;) {
    let inner = node.enter(pos, side, mode,);
    if (!inner) return node;
    node = inner;
  }
}
var TreeNode = class {
  get type() {
    return this._tree.type;
  }
  get name() {
    return this._tree.type.name;
  }
  get to() {
    return this.from + this._tree.length;
  }
  nextChild(i2, dir, pos, side, mode = 0,) {
    for (let parent = this;;) {
      for (let { children, positions, } = parent._tree, e = dir > 0 ? children.length : -1; i2 != e; i2 += dir) {
        let next = children[i2], start = positions[i2] + parent.from;
        if (!checkSide(side, pos, start, start + next.length,)) continue;
        if (next instanceof TreeBuffer) {
          if (mode & IterMode.ExcludeBuffers) continue;
          let index = next.findChild(0, next.buffer.length, dir, pos - start, side,);
          if (index > -1) return new BufferNode(new BufferContext(parent, next, i2, start,), null, index,);
        } else if (mode & IterMode.IncludeAnonymous || !next.type.isAnonymous || hasChild(next,)) {
          let mounted;
          if (!(mode & IterMode.IgnoreMounts) && next.props && (mounted = next.prop(NodeProp.mounted,)) && !mounted.overlay) {
            return new TreeNode(mounted.tree, start, i2, parent,);
          }
          let inner = new TreeNode(next, start, i2, parent,);
          return mode & IterMode.IncludeAnonymous || !inner.type.isAnonymous
            ? inner
            : inner.nextChild(dir < 0 ? next.children.length - 1 : 0, dir, pos, side,);
        }
      }
      if (mode & IterMode.IncludeAnonymous || !parent.type.isAnonymous) return null;
      if (parent.index >= 0) i2 = parent.index + dir;
      else i2 = dir < 0 ? -1 : parent._parent._tree.children.length;
      parent = parent._parent;
      if (!parent) return null;
    }
  }
  get firstChild() {
    return this.nextChild(0, 1, 0, 4,);
  }
  get lastChild() {
    return this.nextChild(this._tree.children.length - 1, -1, 0, 4,);
  }
  childAfter(pos,) {
    return this.nextChild(0, 1, pos, 2,);
  }
  childBefore(pos,) {
    return this.nextChild(this._tree.children.length - 1, -1, pos, -2,);
  }
  enter(pos, side, mode = 0,) {
    let mounted;
    if (!(mode & IterMode.IgnoreOverlays) && (mounted = this._tree.prop(NodeProp.mounted,)) && mounted.overlay) {
      let rPos = pos - this.from;
      for (let { from, to, } of mounted.overlay) {
        if ((side > 0 ? from <= rPos : from < rPos) && (side < 0 ? to >= rPos : to > rPos)) {
          return new TreeNode(mounted.tree, mounted.overlay[0].from + this.from, -1, this,);
        }
      }
    }
    return this.nextChild(0, 1, pos, side, mode,);
  }
  nextSignificantParent() {
    let val = this;
    while (val.type.isAnonymous && val._parent) val = val._parent;
    return val;
  }
  get parent() {
    return this._parent ? this._parent.nextSignificantParent() : null;
  }
  get nextSibling() {
    return this._parent && this.index >= 0 ? this._parent.nextChild(this.index + 1, 1, 0, 4,) : null;
  }
  get prevSibling() {
    return this._parent && this.index >= 0 ? this._parent.nextChild(this.index - 1, -1, 0, 4,) : null;
  }
  cursor(mode = 0,) {
    return new TreeCursor(this, mode,);
  }
  get tree() {
    return this._tree;
  }
  toTree() {
    return this._tree;
  }
  resolve(pos, side = 0,) {
    return resolveNode(this, pos, side, false,);
  }
  resolveInner(pos, side = 0,) {
    return resolveNode(this, pos, side, true,);
  }
  enterUnfinishedNodesBefore(pos,) {
    return enterUnfinishedNodesBefore(this, pos,);
  }
  getChild(type, before = null, after = null,) {
    let r = getChildren(this, type, before, after,);
    return r.length ? r[0] : null;
  }
  getChildren(type, before = null, after = null,) {
    return getChildren(this, type, before, after,);
  }
  /// @internal
  toString() {
    return this._tree.toString();
  }
  get node() {
    return this;
  }
  matchContext(context,) {
    return matchNodeContext(this, context,);
  }
  constructor(_tree, from, index, _parent,) {
    this._tree = _tree;
    this.from = from;
    this.index = index;
    this._parent = _parent;
  }
};
function getChildren(node, type, before, after,) {
  let cur = node.cursor(), result = [];
  if (!cur.firstChild()) return result;
  if (before != null) {
    while (!cur.type.is(before,)) if (!cur.nextSibling()) return result;
  }
  for (;;) {
    if (after != null && cur.type.is(after,)) return result;
    if (cur.type.is(type,)) result.push(cur.node,);
    if (!cur.nextSibling()) return after == null ? result : [];
  }
}
function matchNodeContext(node, context, i2 = context.length - 1,) {
  for (let p = node.parent; i2 >= 0; p = p.parent) {
    if (!p) return false;
    if (!p.type.isAnonymous) {
      if (context[i2] && context[i2] != p.name) return false;
      i2--;
    }
  }
  return true;
}
var BufferContext = class {
  constructor(parent, buffer, index, start,) {
    this.parent = parent;
    this.buffer = buffer;
    this.index = index;
    this.start = start;
  }
};
var BufferNode = class {
  get name() {
    return this.type.name;
  }
  get from() {
    return this.context.start + this.context.buffer.buffer[this.index + 1];
  }
  get to() {
    return this.context.start + this.context.buffer.buffer[this.index + 2];
  }
  child(dir, pos, side,) {
    let { buffer, } = this.context;
    let index = buffer.findChild(this.index + 4, buffer.buffer[this.index + 3], dir, pos - this.context.start, side,);
    return index < 0 ? null : new BufferNode(this.context, this, index,);
  }
  get firstChild() {
    return this.child(1, 0, 4,);
  }
  get lastChild() {
    return this.child(-1, 0, 4,);
  }
  childAfter(pos,) {
    return this.child(1, pos, 2,);
  }
  childBefore(pos,) {
    return this.child(-1, pos, -2,);
  }
  enter(pos, side, mode = 0,) {
    if (mode & IterMode.ExcludeBuffers) return null;
    let { buffer, } = this.context;
    let index = buffer.findChild(this.index + 4, buffer.buffer[this.index + 3], side > 0 ? 1 : -1, pos - this.context.start, side,);
    return index < 0 ? null : new BufferNode(this.context, this, index,);
  }
  get parent() {
    return this._parent || this.context.parent.nextSignificantParent();
  }
  externalSibling(dir,) {
    return this._parent ? null : this.context.parent.nextChild(this.context.index + dir, dir, 0, 4,);
  }
  get nextSibling() {
    let { buffer, } = this.context;
    let after = buffer.buffer[this.index + 3];
    if (after < (this._parent ? buffer.buffer[this._parent.index + 3] : buffer.buffer.length)) {
      return new BufferNode(this.context, this._parent, after,);
    }
    return this.externalSibling(1,);
  }
  get prevSibling() {
    let { buffer, } = this.context;
    let parentStart = this._parent ? this._parent.index + 4 : 0;
    if (this.index == parentStart) return this.externalSibling(-1,);
    return new BufferNode(this.context, this._parent, buffer.findChild(parentStart, this.index, -1, 0, 4,),);
  }
  cursor(mode = 0,) {
    return new TreeCursor(this, mode,);
  }
  get tree() {
    return null;
  }
  toTree() {
    let children = [], positions = [];
    let { buffer, } = this.context;
    let startI = this.index + 4, endI = buffer.buffer[this.index + 3];
    if (endI > startI) {
      let from = buffer.buffer[this.index + 1];
      children.push(buffer.slice(startI, endI, from,),);
      positions.push(0,);
    }
    return new Tree(this.type, children, positions, this.to - this.from,);
  }
  resolve(pos, side = 0,) {
    return resolveNode(this, pos, side, false,);
  }
  resolveInner(pos, side = 0,) {
    return resolveNode(this, pos, side, true,);
  }
  enterUnfinishedNodesBefore(pos,) {
    return enterUnfinishedNodesBefore(this, pos,);
  }
  /// @internal
  toString() {
    return this.context.buffer.childString(this.index,);
  }
  getChild(type, before = null, after = null,) {
    let r = getChildren(this, type, before, after,);
    return r.length ? r[0] : null;
  }
  getChildren(type, before = null, after = null,) {
    return getChildren(this, type, before, after,);
  }
  get node() {
    return this;
  }
  matchContext(context,) {
    return matchNodeContext(this, context,);
  }
  constructor(context, _parent, index,) {
    this.context = context;
    this._parent = _parent;
    this.index = index;
    this.type = context.buffer.set.types[context.buffer.buffer[index]];
  }
};
var TreeCursor = class {
  /// Shorthand for `.type.name`.
  get name() {
    return this.type.name;
  }
  yieldNode(node,) {
    if (!node) return false;
    this._tree = node;
    this.type = node.type;
    this.from = node.from;
    this.to = node.to;
    return true;
  }
  yieldBuf(index, type,) {
    this.index = index;
    let { start, buffer, } = this.buffer;
    this.type = type || buffer.set.types[buffer.buffer[index]];
    this.from = start + buffer.buffer[index + 1];
    this.to = start + buffer.buffer[index + 2];
    return true;
  }
  yield(node,) {
    if (!node) return false;
    if (node instanceof TreeNode) {
      this.buffer = null;
      return this.yieldNode(node,);
    }
    this.buffer = node.context;
    return this.yieldBuf(node.index, node.type,);
  }
  /// @internal
  toString() {
    return this.buffer ? this.buffer.buffer.childString(this.index,) : this._tree.toString();
  }
  /// @internal
  enterChild(dir, pos, side,) {
    if (!this.buffer) {
      return this.yield(this._tree.nextChild(dir < 0 ? this._tree._tree.children.length - 1 : 0, dir, pos, side, this.mode,),);
    }
    let { buffer, } = this.buffer;
    let index = buffer.findChild(this.index + 4, buffer.buffer[this.index + 3], dir, pos - this.buffer.start, side,);
    if (index < 0) return false;
    this.stack.push(this.index,);
    return this.yieldBuf(index,);
  }
  /// Move the cursor to this node's first child. When this returns
  /// false, the node has no child, and the cursor has not been moved.
  firstChild() {
    return this.enterChild(1, 0, 4,);
  }
  /// Move the cursor to this node's last child.
  lastChild() {
    return this.enterChild(-1, 0, 4,);
  }
  /// Move the cursor to the first child that ends after `pos`.
  childAfter(pos,) {
    return this.enterChild(1, pos, 2,);
  }
  /// Move to the last child that starts before `pos`.
  childBefore(pos,) {
    return this.enterChild(-1, pos, -2,);
  }
  /// Move the cursor to the child around `pos`. If side is -1 the
  /// child may end at that position, when 1 it may start there. This
  /// will also enter [overlaid](#common.MountedTree.overlay)
  /// [mounted](#common.NodeProp^mounted) trees unless `overlays` is
  /// set to false.
  enter(pos, side, mode = this.mode,) {
    if (!this.buffer) return this.yield(this._tree.enter(pos, side, mode,),);
    return mode & IterMode.ExcludeBuffers ? false : this.enterChild(1, pos, side,);
  }
  /// Move to the node's parent node, if this isn't the top node.
  parent() {
    if (!this.buffer) return this.yieldNode(this.mode & IterMode.IncludeAnonymous ? this._tree._parent : this._tree.parent,);
    if (this.stack.length) return this.yieldBuf(this.stack.pop(),);
    let parent = this.mode & IterMode.IncludeAnonymous ? this.buffer.parent : this.buffer.parent.nextSignificantParent();
    this.buffer = null;
    return this.yieldNode(parent,);
  }
  /// @internal
  sibling(dir,) {
    if (!this.buffer) {
      return !this._tree._parent
        ? false
        : this.yield(this._tree.index < 0 ? null : this._tree._parent.nextChild(this._tree.index + dir, dir, 0, 4, this.mode,),);
    }
    let { buffer, } = this.buffer, d = this.stack.length - 1;
    if (dir < 0) {
      let parentStart = d < 0 ? 0 : this.stack[d] + 4;
      if (this.index != parentStart) return this.yieldBuf(buffer.findChild(parentStart, this.index, -1, 0, 4,),);
    } else {
      let after = buffer.buffer[this.index + 3];
      if (after < (d < 0 ? buffer.buffer.length : buffer.buffer[this.stack[d] + 3])) return this.yieldBuf(after,);
    }
    return d < 0 ? this.yield(this.buffer.parent.nextChild(this.buffer.index + dir, dir, 0, 4, this.mode,),) : false;
  }
  /// Move to this node's next sibling, if any.
  nextSibling() {
    return this.sibling(1,);
  }
  /// Move to this node's previous sibling, if any.
  prevSibling() {
    return this.sibling(-1,);
  }
  atLastNode(dir,) {
    let index, parent, { buffer, } = this;
    if (buffer) {
      if (dir > 0) {
        if (this.index < buffer.buffer.buffer.length) return false;
      } else {
        for (let i2 = 0; i2 < this.index; i2++) if (buffer.buffer.buffer[i2 + 3] < this.index) return false;
      }
      ({ index, parent, } = buffer);
    } else {
      ({ index, _parent: parent, } = this._tree);
    }
    for (; parent; { index, _parent: parent, } = parent) {
      if (index > -1) {
        for (let i1 = index + dir, e = dir < 0 ? -1 : parent._tree.children.length; i1 != e; i1 += dir) {
          let child = parent._tree.children[i1];
          if (
            this.mode & IterMode.IncludeAnonymous || child instanceof TreeBuffer || !child.type.isAnonymous || hasChild(child,)
          ) return false;
        }
      }
    }
    return true;
  }
  move(dir, enter,) {
    if (enter && this.enterChild(dir, 0, 4,)) return true;
    for (;;) {
      if (this.sibling(dir,)) return true;
      if (this.atLastNode(dir,) || !this.parent()) return false;
    }
  }
  /// Move to the next node in a
  /// [pre-order](https://en.wikipedia.org/wiki/Tree_traversal#Pre-order,_NLR)
  /// traversal, going from a node to its first child or, if the
  /// current node is empty or `enter` is false, its next sibling or
  /// the next sibling of the first parent node that has one.
  next(enter = true,) {
    return this.move(1, enter,);
  }
  /// Move to the next node in a last-to-first pre-order traveral. A
  /// node is followed by its last child or, if it has none, its
  /// previous sibling or the previous sibling of the first parent
  /// node that has one.
  prev(enter = true,) {
    return this.move(-1, enter,);
  }
  /// Move the cursor to the innermost node that covers `pos`. If
  /// `side` is -1, it will enter nodes that end at `pos`. If it is 1,
  /// it will enter nodes that start at `pos`.
  moveTo(pos, side = 0,) {
    while (this.from == this.to || (side < 1 ? this.from >= pos : this.from > pos) || (side > -1 ? this.to <= pos : this.to < pos)) {
      if (!this.parent()) break;
    }
    while (this.enterChild(1, pos, side,)) {
    }
    return this;
  }
  /// Get a [syntax node](#common.SyntaxNode) at the cursor's current
  /// position.
  get node() {
    if (!this.buffer) return this._tree;
    let cache = this.bufferNode, result = null, depth = 0;
    if (cache && cache.context == this.buffer) {
      scan: for (let index = this.index, d = this.stack.length; d >= 0;) {
        for (let c = cache; c; c = c._parent) {
          if (c.index == index) {
            if (index == this.index) return c;
            result = c;
            depth = d + 1;
            break scan;
          }
        }
        index = this.stack[--d];
      }
    }
    for (let i2 = depth; i2 < this.stack.length; i2++) result = new BufferNode(this.buffer, result, this.stack[i2],);
    return this.bufferNode = new BufferNode(this.buffer, result, this.index,);
  }
  /// Get the [tree](#common.Tree) that represents the current node, if
  /// any. Will return null when the node is in a [tree
  /// buffer](#common.TreeBuffer).
  get tree() {
    return this.buffer ? null : this._tree._tree;
  }
  /// Iterate over the current node and all its descendants, calling
  /// `enter` when entering a node and `leave`, if given, when leaving
  /// one. When `enter` returns `false`, any children of that node are
  /// skipped, and `leave` isn't called for it.
  iterate(enter, leave,) {
    for (let depth = 0;;) {
      let mustLeave = false;
      if (this.type.isAnonymous || enter(this,) !== false) {
        if (this.firstChild()) {
          depth++;
          continue;
        }
        if (!this.type.isAnonymous) mustLeave = true;
      }
      for (;;) {
        if (mustLeave && leave) leave(this,);
        mustLeave = this.type.isAnonymous;
        if (this.nextSibling()) break;
        if (!depth) return;
        this.parent();
        depth--;
        mustLeave = true;
      }
    }
  }
  /// Test whether the current node matches a given context—a sequence
  /// of direct parent node names. Empty strings in the context array
  /// are treated as wildcards.
  matchContext(context,) {
    if (!this.buffer) return matchNodeContext(this.node, context,);
    let { buffer, } = this.buffer, { types: types2, } = buffer.set;
    for (let i2 = context.length - 1, d = this.stack.length - 1; i2 >= 0; d--) {
      if (d < 0) return matchNodeContext(this.node, context, i2,);
      let type = types2[buffer.buffer[this.stack[d]]];
      if (!type.isAnonymous) {
        if (context[i2] && context[i2] != type.name) return false;
        i2--;
      }
    }
    return true;
  }
  /// @internal
  constructor(node, mode = 0,) {
    this.mode = mode;
    this.buffer = null;
    this.stack = [];
    this.index = 0;
    this.bufferNode = null;
    if (node instanceof TreeNode) {
      this.yieldNode(node,);
    } else {
      this._tree = node.context.parent;
      this.buffer = node.context;
      for (let n = node._parent; n; n = n._parent) this.stack.unshift(n.index,);
      this.bufferNode = node;
      this.yieldBuf(node.index,);
    }
  }
};
function hasChild(tree,) {
  return tree.children.some((ch,) => ch instanceof TreeBuffer || !ch.type.isAnonymous || hasChild(ch,));
}
function buildTree(data,) {
  var _a2;
  let { buffer, nodeSet: nodeSet2, maxBufferLength = DefaultBufferLength, reused = [], minRepeatType = nodeSet2.types.length, } = data;
  let cursor = Array.isArray(buffer,) ? new FlatBufferCursor(buffer, buffer.length,) : buffer;
  let types2 = nodeSet2.types;
  let contextHash = 0, lookAhead = 0;
  function takeNode(parentStart, minPos, children2, positions2, inRepeat,) {
    let { id: id2, start, end, size, } = cursor;
    let lookAheadAtStart = lookAhead;
    while (size < 0) {
      cursor.next();
      if (size == -1) {
        let node2 = reused[id2];
        children2.push(node2,);
        positions2.push(start - parentStart,);
        return;
      } else if (size == -3) {
        contextHash = id2;
        return;
      } else if (size == -4) {
        lookAhead = id2;
        return;
      } else {
        throw new RangeError(`Unrecognized record size: ${size}`,);
      }
    }
    let type = types2[id2], node, buffer2;
    let startPos = start - parentStart;
    if (end - start <= maxBufferLength && (buffer2 = findBufferSize(cursor.pos - minPos, inRepeat,))) {
      let data2 = new Uint16Array(buffer2.size - buffer2.skip,);
      let endPos = cursor.pos - buffer2.size, index = data2.length;
      while (cursor.pos > endPos) index = copyToBuffer(buffer2.start, data2, index,);
      node = new TreeBuffer(data2, end - buffer2.start, nodeSet2,);
      startPos = buffer2.start - parentStart;
    } else {
      let endPos1 = cursor.pos - size;
      cursor.next();
      let localChildren = [], localPositions = [];
      let localInRepeat = id2 >= minRepeatType ? id2 : -1;
      let lastGroup = 0, lastEnd = end;
      while (cursor.pos > endPos1) {
        if (localInRepeat >= 0 && cursor.id == localInRepeat && cursor.size >= 0) {
          if (cursor.end <= lastEnd - maxBufferLength) {
            makeRepeatLeaf(localChildren, localPositions, start, lastGroup, cursor.end, lastEnd, localInRepeat, lookAheadAtStart,);
            lastGroup = localChildren.length;
            lastEnd = cursor.end;
          }
          cursor.next();
        } else {
          takeNode(start, endPos1, localChildren, localPositions, localInRepeat,);
        }
      }
      if (localInRepeat >= 0 && lastGroup > 0 && lastGroup < localChildren.length) {
        makeRepeatLeaf(localChildren, localPositions, start, lastGroup, start, lastEnd, localInRepeat, lookAheadAtStart,);
      }
      localChildren.reverse();
      localPositions.reverse();
      if (localInRepeat > -1 && lastGroup > 0) {
        let make = makeBalanced(type,);
        node = balanceRange(type, localChildren, localPositions, 0, localChildren.length, 0, end - start, make, make,);
      } else {
        node = makeTree(type, localChildren, localPositions, end - start, lookAheadAtStart - end,);
      }
    }
    children2.push(node,);
    positions2.push(startPos,);
  }
  function makeBalanced(type,) {
    return (children2, positions2, length2,) => {
      let lookAhead2 = 0, lastI = children2.length - 1, last, lookAheadProp;
      if (lastI >= 0 && (last = children2[lastI]) instanceof Tree) {
        if (!lastI && last.type == type && last.length == length2) return last;
        if (lookAheadProp = last.prop(NodeProp.lookAhead,)) lookAhead2 = positions2[lastI] + last.length + lookAheadProp;
      }
      return makeTree(type, children2, positions2, length2, lookAhead2,);
    };
  }
  function makeRepeatLeaf(children2, positions2, base2, i2, from, to, type, lookAhead2,) {
    let localChildren = [], localPositions = [];
    while (children2.length > i2) {
      localChildren.push(children2.pop(),);
      localPositions.push(positions2.pop() + base2 - from,);
    }
    children2.push(makeTree(nodeSet2.types[type], localChildren, localPositions, to - from, lookAhead2 - to,),);
    positions2.push(from - base2,);
  }
  function makeTree(type, children2, positions2, length2, lookAhead2 = 0, props,) {
    if (contextHash) {
      let pair2 = [NodeProp.contextHash, contextHash,];
      props = props ? [pair2,].concat(props,) : [pair2,];
    }
    if (lookAhead2 > 25) {
      let pair1 = [NodeProp.lookAhead, lookAhead2,];
      props = props ? [pair1,].concat(props,) : [pair1,];
    }
    return new Tree(type, children2, positions2, length2, props,);
  }
  function findBufferSize(maxSize, inRepeat,) {
    let fork = cursor.fork();
    let size = 0, start = 0, skip = 0, minStart = fork.end - maxBufferLength;
    let result = { size: 0, start: 0, skip: 0, };
    scan: for (let minPos = fork.pos - maxSize; fork.pos > minPos;) {
      let nodeSize2 = fork.size;
      if (fork.id == inRepeat && nodeSize2 >= 0) {
        result.size = size;
        result.start = start;
        result.skip = skip;
        skip += 4;
        size += 4;
        fork.next();
        continue;
      }
      let startPos = fork.pos - nodeSize2;
      if (nodeSize2 < 0 || startPos < minPos || fork.start < minStart) break;
      let localSkipped = fork.id >= minRepeatType ? 4 : 0;
      let nodeStart = fork.start;
      fork.next();
      while (fork.pos > startPos) {
        if (fork.size < 0) {
          if (fork.size == -3) localSkipped += 4;
          else break scan;
        } else if (fork.id >= minRepeatType) {
          localSkipped += 4;
        }
        fork.next();
      }
      start = nodeStart;
      size += nodeSize2;
      skip += localSkipped;
    }
    if (inRepeat < 0 || size == maxSize) {
      result.size = size;
      result.start = start;
      result.skip = skip;
    }
    return result.size > 4 ? result : void 0;
  }
  function copyToBuffer(bufferStart, buffer2, index,) {
    let { id: id2, start, end, size, } = cursor;
    cursor.next();
    if (size >= 0 && id2 < minRepeatType) {
      let startIndex = index;
      if (size > 4) {
        let endPos = cursor.pos - (size - 4);
        while (cursor.pos > endPos) index = copyToBuffer(bufferStart, buffer2, index,);
      }
      buffer2[--index] = startIndex;
      buffer2[--index] = end - bufferStart;
      buffer2[--index] = start - bufferStart;
      buffer2[--index] = id2;
    } else if (size == -3) {
      contextHash = id2;
    } else if (size == -4) {
      lookAhead = id2;
    }
    return index;
  }
  let children = [], positions = [];
  while (cursor.pos > 0) takeNode(data.start || 0, data.bufferStart || 0, children, positions, -1,);
  let length = (_a2 = data.length) !== null && _a2 !== void 0 ? _a2 : children.length ? positions[0] + children[0].length : 0;
  return new Tree(types2[data.topID], children.reverse(), positions.reverse(), length,);
}
var nodeSizeCache = /* @__PURE__ */ new WeakMap();
function nodeSize(balanceType, node,) {
  if (!balanceType.isAnonymous || node instanceof TreeBuffer || node.type != balanceType) return 1;
  let size = nodeSizeCache.get(node,);
  if (size == null) {
    size = 1;
    for (let child of node.children) {
      if (child.type != balanceType || !(child instanceof Tree)) {
        size = 1;
        break;
      }
      size += nodeSize(balanceType, child,);
    }
    nodeSizeCache.set(node, size,);
  }
  return size;
}
function balanceRange(balanceType, children, positions, from, to, start, length, mkTop, mkTree,) {
  let total = 0;
  for (let i2 = from; i2 < to; i2++) total += nodeSize(balanceType, children[i2],);
  let maxChild = Math.ceil(total * 1.5 / 8,);
  let localChildren = [], localPositions = [];
  function divide(children2, positions2, from2, to2, offset,) {
    for (let i2 = from2; i2 < to2;) {
      let groupFrom = i2, groupStart = positions2[i2], groupSize = nodeSize(balanceType, children2[i2],);
      i2++;
      for (; i2 < to2; i2++) {
        let nextSize = nodeSize(balanceType, children2[i2],);
        if (groupSize + nextSize >= maxChild) break;
        groupSize += nextSize;
      }
      if (i2 == groupFrom + 1) {
        if (groupSize > maxChild) {
          let only = children2[groupFrom];
          divide(only.children, only.positions, 0, only.children.length, positions2[groupFrom] + offset,);
          continue;
        }
        localChildren.push(children2[groupFrom],);
      } else {
        let length2 = positions2[i2 - 1] + children2[i2 - 1].length - groupStart;
        localChildren.push(balanceRange(balanceType, children2, positions2, groupFrom, i2, groupStart, length2, null, mkTree,),);
      }
      localPositions.push(groupStart + offset - start,);
    }
  }
  divide(children, positions, from, to, 0,);
  return (mkTop || mkTree)(localChildren, localPositions, length,);
}
var NodeWeakMap = class {
  setBuffer(buffer, index, value,) {
    let inner = this.map.get(buffer,);
    if (!inner) this.map.set(buffer, inner = /* @__PURE__ */ new Map(),);
    inner.set(index, value,);
  }
  getBuffer(buffer, index,) {
    let inner = this.map.get(buffer,);
    return inner && inner.get(index,);
  }
  /// Set the value for this syntax node.
  set(node, value,) {
    if (node instanceof BufferNode) this.setBuffer(node.context.buffer, node.index, value,);
    else if (node instanceof TreeNode) this.map.set(node.tree, value,);
  }
  /// Retrieve value for this syntax node, if it exists in the map.
  get(node,) {
    return node instanceof BufferNode
      ? this.getBuffer(node.context.buffer, node.index,)
      : node instanceof TreeNode
      ? this.map.get(node.tree,)
      : void 0;
  }
  /// Set the value for the node that a cursor currently points to.
  cursorSet(cursor, value,) {
    if (cursor.buffer) this.setBuffer(cursor.buffer.buffer, cursor.index, value,);
    else this.map.set(cursor.tree, value,);
  }
  /// Retrieve the value for the node that a cursor currently points
  /// to.
  cursorGet(cursor,) {
    return cursor.buffer ? this.getBuffer(cursor.buffer.buffer, cursor.index,) : this.map.get(cursor.tree,);
  }
  constructor() {
    this.map = /* @__PURE__ */ new WeakMap();
  }
};
var TreeFragment = class {
  /// Whether the start of the fragment represents the start of a
  /// parse, or the end of a change. (In the second case, it may not
  /// be safe to reuse some nodes at the start, depending on the
  /// parsing algorithm.)
  get openStart() {
    return (this.open & 1) > 0;
  }
  /// Whether the end of the fragment represents the end of a
  /// full-document parse, or the start of a change.
  get openEnd() {
    return (this.open & 2) > 0;
  }
  /// Create a set of fragments from a freshly parsed tree, or update
  /// an existing set of fragments by replacing the ones that overlap
  /// with a tree with content from the new tree. When `partial` is
  /// true, the parse is treated as incomplete, and the resulting
  /// fragment has [`openEnd`](#common.TreeFragment.openEnd) set to
  /// true.
  static addTree(tree, fragments = [], partial = false,) {
    let result = [new TreeFragment(0, tree.length, tree, 0, false, partial,),];
    for (let f of fragments) if (f.to > tree.length) result.push(f,);
    return result;
  }
  /// Apply a set of edits to an array of fragments, removing or
  /// splitting fragments as necessary to remove edited ranges, and
  /// adjusting offsets for fragments that moved.
  static applyChanges(fragments, changes, minGap = 128,) {
    if (!changes.length) return fragments;
    let result = [];
    let fI = 1, nextF = fragments.length ? fragments[0] : null;
    for (let cI = 0, pos = 0, off = 0;; cI++) {
      let nextC = cI < changes.length ? changes[cI] : null;
      let nextPos = nextC ? nextC.fromA : 1e9;
      if (nextPos - pos >= minGap) {
        while (nextF && nextF.from < nextPos) {
          let cut = nextF;
          if (pos >= cut.from || nextPos <= cut.to || off) {
            let fFrom = Math.max(cut.from, pos,) - off, fTo = Math.min(cut.to, nextPos,) - off;
            cut = fFrom >= fTo ? null : new TreeFragment(fFrom, fTo, cut.tree, cut.offset + off, cI > 0, !!nextC,);
          }
          if (cut) result.push(cut,);
          if (nextF.to > nextPos) break;
          nextF = fI < fragments.length ? fragments[fI++] : null;
        }
      }
      if (!nextC) break;
      pos = nextC.toA;
      off = nextC.toA - nextC.toB;
    }
    return result;
  }
  /// Construct a tree fragment. You'll usually want to use
  /// [`addTree`](#common.TreeFragment^addTree) and
  /// [`applyChanges`](#common.TreeFragment^applyChanges) instead of
  /// calling this directly.
  constructor(from, to, tree, offset, openStart = false, openEnd = false,) {
    this.from = from;
    this.to = to;
    this.tree = tree;
    this.offset = offset;
    this.open = (openStart ? 1 : 0) | (openEnd ? 2 : 0);
  }
};
var Parser = class {
  /// Start a parse, returning a [partial parse](#common.PartialParse)
  /// object. [`fragments`](#common.TreeFragment) can be passed in to
  /// make the parse incremental.
  ///
  /// By default, the entire input is parsed. You can pass `ranges`,
  /// which should be a sorted array of non-empty, non-overlapping
  /// ranges, to parse only those ranges. The tree returned in that
  /// case will start at `ranges[0].from`.
  startParse(input, fragments, ranges,) {
    if (typeof input == 'string') input = new StringInput(input,);
    ranges = !ranges
      ? [new Range2(0, input.length,),]
      : ranges.length
      ? ranges.map((r,) => new Range2(r.from, r.to,))
      : [new Range2(0, 0,),];
    return this.createParse(input, fragments || [], ranges,);
  }
  /// Run a full parse, returning the resulting tree.
  parse(input, fragments, ranges,) {
    let parse = this.startParse(input, fragments, ranges,);
    for (;;) {
      let done = parse.advance();
      if (done) return done;
    }
  }
};
var StringInput = class {
  get length() {
    return this.string.length;
  }
  chunk(from,) {
    return this.string.slice(from,);
  }
  get lineChunks() {
    return false;
  }
  read(from, to,) {
    return this.string.slice(from, to,);
  }
  constructor(string2,) {
    this.string = string2;
  }
};
function parseMixed(nest,) {
  return (parse, input, fragments, ranges,) => new MixedParse(parse, nest, input, fragments, ranges,);
}
var InnerParse = class {
  constructor(parser, parse, overlay, target, ranges,) {
    this.parser = parser;
    this.parse = parse;
    this.overlay = overlay;
    this.target = target;
    this.ranges = ranges;
    if (!ranges.length || ranges.some((r,) => r.from >= r.to)) {
      throw new RangeError('Invalid inner parse ranges given: ' + JSON.stringify(ranges,),);
    }
  }
};
var ActiveOverlay = class {
  constructor(parser, predicate, mounts, index, start, target, prev,) {
    this.parser = parser;
    this.predicate = predicate;
    this.mounts = mounts;
    this.index = index;
    this.start = start;
    this.target = target;
    this.prev = prev;
    this.depth = 0;
    this.ranges = [];
  }
};
var stoppedInner = new NodeProp({ perNode: true, },);
var MixedParse = class {
  advance() {
    if (this.baseParse) {
      let done2 = this.baseParse.advance();
      if (!done2) return null;
      this.baseParse = null;
      this.baseTree = done2;
      this.startInner();
      if (this.stoppedAt != null) for (let inner2 of this.inner) inner2.parse.stopAt(this.stoppedAt,);
    }
    if (this.innerDone == this.inner.length) {
      let result = this.baseTree;
      if (this.stoppedAt != null) {
        result = new Tree(
          result.type,
          result.children,
          result.positions,
          result.length,
          result.propValues.concat([[stoppedInner, this.stoppedAt,],],),
        );
      }
      return result;
    }
    let inner = this.inner[this.innerDone], done = inner.parse.advance();
    if (done) {
      this.innerDone++;
      let props = Object.assign(/* @__PURE__ */ Object.create(null,), inner.target.props,);
      props[NodeProp.mounted.id] = new MountedTree(done, inner.overlay, inner.parser,);
      inner.target.props = props;
    }
    return null;
  }
  get parsedPos() {
    if (this.baseParse) return 0;
    let pos = this.input.length;
    for (let i2 = this.innerDone; i2 < this.inner.length; i2++) {
      if (this.inner[i2].ranges[0].from < pos) pos = Math.min(pos, this.inner[i2].parse.parsedPos,);
    }
    return pos;
  }
  stopAt(pos,) {
    this.stoppedAt = pos;
    if (this.baseParse) this.baseParse.stopAt(pos,);
    else for (let i2 = this.innerDone; i2 < this.inner.length; i2++) this.inner[i2].parse.stopAt(pos,);
  }
  startInner() {
    let fragmentCursor = new FragmentCursor(this.fragments,);
    let overlay = null;
    let covered = null;
    let cursor = new TreeCursor(
      new TreeNode(this.baseTree, this.ranges[0].from, 0, null,),
      IterMode.IncludeAnonymous | IterMode.IgnoreMounts,
    );
    scan: for (let nest, isCovered; this.stoppedAt == null || cursor.from < this.stoppedAt;) {
      let enter = true, range;
      if (fragmentCursor.hasNode(cursor,)) {
        if (overlay) {
          let match = overlay.mounts.find((m,) => m.frag.from <= cursor.from && m.frag.to >= cursor.to && m.mount.overlay);
          if (match) {
            for (let r of match.mount.overlay) {
              let from = r.from + match.pos, to = r.to + match.pos;
              if (from >= cursor.from && to <= cursor.to && !overlay.ranges.some((r2,) => r2.from < to && r2.to > from)) {
                overlay.ranges.push({ from, to, },);
              }
            }
          }
        }
        enter = false;
      } else if (covered && (isCovered = checkCover(covered.ranges, cursor.from, cursor.to,))) {
        enter = isCovered != 2;
      } else if (!cursor.type.isAnonymous && cursor.from < cursor.to && (nest = this.nest(cursor, this.input,))) {
        if (!cursor.tree) materialize(cursor,);
        let oldMounts = fragmentCursor.findMounts(cursor.from, nest.parser,);
        if (typeof nest.overlay == 'function') {
          overlay = new ActiveOverlay(nest.parser, nest.overlay, oldMounts, this.inner.length, cursor.from, cursor.tree, overlay,);
        } else {
          let ranges = punchRanges(this.ranges, nest.overlay || [new Range2(cursor.from, cursor.to,),],);
          if (ranges.length) {
            this.inner.push(
              new InnerParse(
                nest.parser,
                nest.parser.startParse(this.input, enterFragments(oldMounts, ranges,), ranges,),
                nest.overlay
                  ? nest.overlay.map((r,) => new Range2(r.from - cursor.from, r.to - cursor.from,))
                  : null,
                cursor.tree,
                ranges,
              ),
            );
          }
          if (!nest.overlay) enter = false;
          else if (ranges.length) covered = { ranges, depth: 0, prev: covered, };
        }
      } else if (overlay && (range = overlay.predicate(cursor,))) {
        if (range === true) range = new Range2(cursor.from, cursor.to,);
        if (range.from < range.to) overlay.ranges.push(range,);
      }
      if (enter && cursor.firstChild()) {
        if (overlay) overlay.depth++;
        if (covered) covered.depth++;
      } else {
        for (;;) {
          if (cursor.nextSibling()) break;
          if (!cursor.parent()) break scan;
          if (overlay && !--overlay.depth) {
            let ranges1 = punchRanges(this.ranges, overlay.ranges,);
            if (ranges1.length) {
              this.inner.splice(
                overlay.index,
                0,
                new InnerParse(
                  overlay.parser,
                  overlay.parser.startParse(this.input, enterFragments(overlay.mounts, ranges1,), ranges1,),
                  overlay.ranges.map((r,) => new Range2(r.from - overlay.start, r.to - overlay.start,)),
                  overlay.target,
                  ranges1,
                ),
              );
            }
            overlay = overlay.prev;
          }
          if (covered && !--covered.depth) covered = covered.prev;
        }
      }
    }
  }
  constructor(base2, nest, input, fragments, ranges,) {
    this.nest = nest;
    this.input = input;
    this.fragments = fragments;
    this.ranges = ranges;
    this.inner = [];
    this.innerDone = 0;
    this.baseTree = null;
    this.stoppedAt = null;
    this.baseParse = base2;
  }
};
function checkCover(covered, from, to,) {
  for (let range of covered) {
    if (range.from >= to) break;
    if (range.to > from) return range.from <= from && range.to >= to ? 2 : 1;
  }
  return 0;
}
function sliceBuf(buf, startI, endI, nodes, positions, off,) {
  if (startI < endI) {
    let from = buf.buffer[startI + 1];
    nodes.push(buf.slice(startI, endI, from,),);
    positions.push(from - off,);
  }
}
function materialize(cursor,) {
  let { node, } = cursor, depth = 0;
  do {
    cursor.parent();
    depth++;
  } while (!cursor.tree);
  let i2 = 0, base2 = cursor.tree, off = 0;
  for (;; i2++) {
    off = base2.positions[i2] + cursor.from;
    if (off <= node.from && off + base2.children[i2].length >= node.to) break;
  }
  let buf = base2.children[i2], b = buf.buffer;
  function split(startI, endI, type, innerOffset, length,) {
    let i22 = startI;
    while (b[i22 + 2] + off <= node.from) i22 = b[i22 + 3];
    let children = [], positions = [];
    sliceBuf(buf, startI, i22, children, positions, innerOffset,);
    let from = b[i22 + 1], to = b[i22 + 2];
    let isTarget = from + off == node.from && to + off == node.to && b[i22] == node.type.id;
    children.push(isTarget ? node.toTree() : split(i22 + 4, b[i22 + 3], buf.set.types[b[i22]], from, to - from,),);
    positions.push(from - innerOffset,);
    sliceBuf(buf, b[i22 + 3], endI, children, positions, innerOffset,);
    return new Tree(type, children, positions, length,);
  }
  base2.children[i2] = split(0, b.length, NodeType.none, 0, buf.length,);
  for (let d = 0; d <= depth; d++) cursor.childAfter(node.from,);
}
var StructureCursor = class {
  // Move to the first node (in pre-order) that starts at or after `pos`.
  moveTo(pos,) {
    let { cursor, } = this, p = pos - this.offset;
    while (!this.done && cursor.from < p) {
      if (cursor.to >= pos && cursor.enter(p, 1, IterMode.IgnoreOverlays | IterMode.ExcludeBuffers,));
      else if (!cursor.next(false,)) this.done = true;
    }
  }
  hasNode(cursor,) {
    this.moveTo(cursor.from,);
    if (!this.done && this.cursor.from + this.offset == cursor.from && this.cursor.tree) {
      for (let tree = this.cursor.tree;;) {
        if (tree == cursor.tree) return true;
        if (tree.children.length && tree.positions[0] == 0 && tree.children[0] instanceof Tree) tree = tree.children[0];
        else break;
      }
    }
    return false;
  }
  constructor(root, offset,) {
    this.offset = offset;
    this.done = false;
    this.cursor = root.cursor(IterMode.IncludeAnonymous | IterMode.IgnoreMounts,);
  }
};
var FragmentCursor = class {
  hasNode(node,) {
    while (this.curFrag && node.from >= this.curTo) this.nextFrag();
    return this.curFrag && this.curFrag.from <= node.from && this.curTo >= node.to && this.inner.hasNode(node,);
  }
  nextFrag() {
    var _a2;
    this.fragI++;
    if (this.fragI == this.fragments.length) {
      this.curFrag = this.inner = null;
    } else {
      let frag = this.curFrag = this.fragments[this.fragI];
      this.curTo = (_a2 = frag.tree.prop(stoppedInner,)) !== null && _a2 !== void 0 ? _a2 : frag.to;
      this.inner = new StructureCursor(frag.tree, -frag.offset,);
    }
  }
  findMounts(pos, parser,) {
    var _a2;
    let result = [];
    if (this.inner) {
      this.inner.cursor.moveTo(pos, 1,);
      for (let pos2 = this.inner.cursor.node; pos2; pos2 = pos2.parent) {
        let mount = (_a2 = pos2.tree) === null || _a2 === void 0 ? void 0 : _a2.prop(NodeProp.mounted,);
        if (mount && mount.parser == parser) {
          for (let i2 = this.fragI; i2 < this.fragments.length; i2++) {
            let frag = this.fragments[i2];
            if (frag.from >= pos2.to) break;
            if (frag.tree == this.curFrag.tree) result.push({ frag, pos: pos2.from - frag.offset, mount, },);
          }
        }
      }
    }
    return result;
  }
  constructor(fragments,) {
    var _a2;
    this.fragments = fragments;
    this.curTo = 0;
    this.fragI = 0;
    if (fragments.length) {
      let first = this.curFrag = fragments[0];
      this.curTo = (_a2 = first.tree.prop(stoppedInner,)) !== null && _a2 !== void 0 ? _a2 : first.to;
      this.inner = new StructureCursor(first.tree, -first.offset,);
    } else {
      this.curFrag = this.inner = null;
    }
  }
};
function punchRanges(outer, ranges,) {
  let copy = null, current = ranges;
  for (let i2 = 1, j = 0; i2 < outer.length; i2++) {
    let gapFrom = outer[i2 - 1].to, gapTo = outer[i2].from;
    for (; j < current.length; j++) {
      let r = current[j];
      if (r.from >= gapTo) break;
      if (r.to <= gapFrom) continue;
      if (!copy) current = copy = ranges.slice();
      if (r.from < gapFrom) {
        copy[j] = new Range2(r.from, gapFrom,);
        if (r.to > gapTo) copy.splice(j + 1, 0, new Range2(gapTo, r.to,),);
      } else if (r.to > gapTo) {
        copy[j--] = new Range2(gapTo, r.to,);
      } else {
        copy.splice(j--, 1,);
      }
    }
  }
  return current;
}
function findCoverChanges(a, b, from, to,) {
  let iA = 0, iB = 0, inA = false, inB = false, pos = -1e9;
  let result = [];
  for (;;) {
    let nextA = iA == a.length ? 1e9 : inA ? a[iA].to : a[iA].from;
    let nextB = iB == b.length ? 1e9 : inB ? b[iB].to : b[iB].from;
    if (inA != inB) {
      let start = Math.max(pos, from,), end = Math.min(nextA, nextB, to,);
      if (start < end) result.push(new Range2(start, end,),);
    }
    pos = Math.min(nextA, nextB,);
    if (pos == 1e9) break;
    if (nextA == pos) {
      if (!inA) inA = true;
      else {
        inA = false;
        iA++;
      }
    }
    if (nextB == pos) {
      if (!inB) inB = true;
      else {
        inB = false;
        iB++;
      }
    }
  }
  return result;
}
function enterFragments(mounts, ranges,) {
  let result = [];
  for (let { pos, mount, frag, } of mounts) {
    let startPos = pos + (mount.overlay ? mount.overlay[0].from : 0), endPos = startPos + mount.tree.length;
    let from = Math.max(frag.from, startPos,), to = Math.min(frag.to, endPos,);
    if (mount.overlay) {
      let overlay = mount.overlay.map((r,) => new Range2(r.from + pos, r.to + pos,));
      let changes = findCoverChanges(ranges, overlay, from, to,);
      for (let i2 = 0, pos2 = from;; i2++) {
        let last = i2 == changes.length, end = last ? to : changes[i2].from;
        if (end > pos2) {
          result.push(
            new TreeFragment(pos2, end, mount.tree, -startPos, frag.from >= pos2 || frag.openStart, frag.to <= end || frag.openEnd,),
          );
        }
        if (last) break;
        pos2 = changes[i2].to;
      }
    } else {
      result.push(
        new TreeFragment(from, to, mount.tree, -startPos, frag.from >= startPos || frag.openStart, frag.to <= endPos || frag.openEnd,),
      );
    }
  }
  return result;
}

// https :https://framerusercontent.com/modules/rOWwbZHN39cczduPnzmw/OTf12FftmvJsuTNsZb3G/lezer_highlight.js
var nextTagID = 0;
var Tag = class {
  /**
  Define a new tag. If `parent` is given, the tag is treated as a
  sub-tag of that parent, and
  [highlighters](#highlight.tagHighlighter) that don't mention
  this tag will try to fall back to the parent tag (or grandparent
  tag, etc).
  */
  static define(parent,) {
    if (parent === null || parent === void 0 ? void 0 : parent.base) throw new Error('Can not derive from a modified tag',);
    let tag = new Tag([], null, [],);
    tag.set.push(tag,);
    if (parent) for (let t2 of parent.set) tag.set.push(t2,);
    return tag;
  }
  /**
  Define a tag _modifier_, which is a function that, given a tag,
  will return a tag that is a subtag of the original. Applying the
  same modifier to a twice tag will return the same value (`m1(t1)
  == m1(t1)`) and applying multiple modifiers will, regardless or
  order, produce the same tag (`m1(m2(t1)) == m2(m1(t1))`).

  When multiple modifiers are applied to a given base tag, each
  smaller set of modifiers is registered as a parent, so that for
  example `m1(m2(m3(t1)))` is a subtype of `m1(m2(t1))`,
  `m1(m3(t1)`, and so on.
  */
  static defineModifier() {
    let mod = new Modifier();
    return (tag,) => {
      if (tag.modified.indexOf(mod,) > -1) return tag;
      return Modifier.get(tag.base || tag, tag.modified.concat(mod,).sort((a, b,) => a.id - b.id),);
    };
  }
  /**
  @internal
  */
  constructor(set, base2, modified,) {
    this.set = set;
    this.base = base2;
    this.modified = modified;
    this.id = nextTagID++;
  }
};
var nextModifierID = 0;
var Modifier = class {
  static get(base2, mods,) {
    if (!mods.length) return base2;
    let exists = mods[0].instances.find((t2,) => t2.base == base2 && sameArray2(mods, t2.modified,));
    if (exists) return exists;
    let set = [], tag = new Tag(set, base2, mods,);
    for (let m of mods) m.instances.push(tag,);
    let configs = powerSet(mods,);
    for (let parent of base2.set) if (!parent.modified.length) for (let config of configs) set.push(Modifier.get(parent, config,),);
    return tag;
  }
  constructor() {
    this.instances = [];
    this.id = nextModifierID++;
  }
};
function sameArray2(a, b,) {
  return a.length == b.length && a.every((x, i2,) => x == b[i2]);
}
function powerSet(array,) {
  let sets = [[],];
  for (let i2 = 0; i2 < array.length; i2++) {
    for (let j = 0, e = sets.length; j < e; j++) {
      sets.push(sets[j].concat(array[i2],),);
    }
  }
  return sets.sort((a, b,) => b.length - a.length);
}
function styleTags(spec,) {
  let byName = /* @__PURE__ */ Object.create(null,);
  for (let prop in spec) {
    let tags2 = spec[prop];
    if (!Array.isArray(tags2,)) tags2 = [tags2,];
    for (let part of prop.split(' ',)) {
      if (part) {
        let pieces = [], mode = 2, rest = part;
        for (let pos = 0;;) {
          if (rest == '...' && pos > 0 && pos + 3 == part.length) {
            mode = 1;
            break;
          }
          let m = /^"(?:[^"\\]|\\.)*?"|[^\/!]+/.exec(rest,);
          if (!m) throw new RangeError('Invalid path: ' + part,);
          pieces.push(m[0] == '*' ? '' : m[0][0] == '"' ? JSON.parse(m[0],) : m[0],);
          pos += m[0].length;
          if (pos == part.length) break;
          let next = part[pos++];
          if (pos == part.length && next == '!') {
            mode = 0;
            break;
          }
          if (next != '/') throw new RangeError('Invalid path: ' + part,);
          rest = part.slice(pos,);
        }
        let last = pieces.length - 1, inner = pieces[last];
        if (!inner) throw new RangeError('Invalid path: ' + part,);
        let rule = new Rule(tags2, mode, last > 0 ? pieces.slice(0, last,) : null,);
        byName[inner] = rule.sort(byName[inner],);
      }
    }
  }
  return ruleNodeProp.add(byName,);
}
var ruleNodeProp = new NodeProp();
var Rule = class {
  get opaque() {
    return this.mode == 0;
  }
  get inherit() {
    return this.mode == 1;
  }
  sort(other,) {
    if (!other || other.depth < this.depth) {
      this.next = other;
      return this;
    }
    other.next = this.sort(other.next,);
    return other;
  }
  get depth() {
    return this.context ? this.context.length : 0;
  }
  constructor(tags2, mode, context, next,) {
    this.tags = tags2;
    this.mode = mode;
    this.context = context;
    this.next = next;
  }
};
Rule.empty = new Rule([], 2, null,);
function tagHighlighter(tags2, options,) {
  let map = /* @__PURE__ */ Object.create(null,);
  for (let style of tags2) {
    if (!Array.isArray(style.tag,)) map[style.tag.id] = style.class;
    else for (let tag of style.tag) map[tag.id] = style.class;
  }
  let { scope, all = null, } = options || {};
  return {
    style: (tags3,) => {
      let cls = all;
      for (let tag of tags3) {
        for (let sub of tag.set) {
          let tagClass = map[sub.id];
          if (tagClass) {
            cls = cls ? cls + ' ' + tagClass : tagClass;
            break;
          }
        }
      }
      return cls;
    },
    scope,
  };
}
function highlightTags(highlighters, tags2,) {
  let result = null;
  for (let highlighter of highlighters) {
    let value = highlighter.style(tags2,);
    if (value) result = result ? result + ' ' + value : value;
  }
  return result;
}
function highlightTree(tree, highlighter, putStyle, from = 0, to = tree.length,) {
  let builder = new HighlightBuilder(from, Array.isArray(highlighter,) ? highlighter : [highlighter,], putStyle,);
  builder.highlightRange(tree.cursor(), from, to, '', builder.highlighters,);
  builder.flush(to,);
}
var HighlightBuilder = class {
  startSpan(at, cls,) {
    if (cls != this.class) {
      this.flush(at,);
      if (at > this.at) this.at = at;
      this.class = cls;
    }
  }
  flush(to,) {
    if (to > this.at && this.class) this.span(this.at, to, this.class,);
  }
  highlightRange(cursor, from, to, inheritedClass, highlighters,) {
    let { type, from: start, to: end, } = cursor;
    if (start >= to || end <= from) return;
    if (type.isTop) highlighters = this.highlighters.filter((h,) => !h.scope || h.scope(type,));
    let cls = inheritedClass;
    let rule = getStyleTags(cursor,) || Rule.empty;
    let tagCls = highlightTags(highlighters, rule.tags,);
    if (tagCls) {
      if (cls) cls += ' ';
      cls += tagCls;
      if (rule.mode == 1) inheritedClass += (inheritedClass ? ' ' : '') + tagCls;
    }
    this.startSpan(Math.max(from, start,), cls,);
    if (rule.opaque) return;
    let mounted = cursor.tree && cursor.tree.prop(NodeProp.mounted,);
    if (mounted && mounted.overlay) {
      let inner = cursor.node.enter(mounted.overlay[0].from + start, 1,);
      let innerHighlighters = this.highlighters.filter((h,) => !h.scope || h.scope(mounted.tree.type,));
      let hasChild2 = cursor.firstChild();
      for (let i2 = 0, pos = start;; i2++) {
        let next = i2 < mounted.overlay.length ? mounted.overlay[i2] : null;
        let nextPos = next ? next.from + start : end;
        let rangeFrom = Math.max(from, pos,), rangeTo = Math.min(to, nextPos,);
        if (rangeFrom < rangeTo && hasChild2) {
          while (cursor.from < rangeTo) {
            this.highlightRange(cursor, rangeFrom, rangeTo, inheritedClass, highlighters,);
            this.startSpan(Math.min(rangeTo, cursor.to,), cls,);
            if (cursor.to >= nextPos || !cursor.nextSibling()) break;
          }
        }
        if (!next || nextPos > to) break;
        pos = next.to + start;
        if (pos > from) {
          this.highlightRange(inner.cursor(), Math.max(from, next.from + start,), Math.min(to, pos,), '', innerHighlighters,);
          this.startSpan(Math.min(to, pos,), cls,);
        }
      }
      if (hasChild2) cursor.parent();
    } else if (cursor.firstChild()) {
      if (mounted) inheritedClass = '';
      do {
        if (cursor.to <= from) continue;
        if (cursor.from >= to) break;
        this.highlightRange(cursor, from, to, inheritedClass, highlighters,);
        this.startSpan(Math.min(to, cursor.to,), cls,);
      } while (cursor.nextSibling());
      cursor.parent();
    }
  }
  constructor(at, highlighters, span,) {
    this.at = at;
    this.highlighters = highlighters;
    this.span = span;
    this.class = '';
  }
};
function getStyleTags(node,) {
  let rule = node.type.prop(ruleNodeProp,);
  while (rule && rule.context && !node.matchContext(rule.context,)) rule = rule.next;
  return rule || null;
}
var t = Tag.define;
var comment = t();
var name = t();
var typeName = t(name,);
var propertyName = t(name,);
var literal = t();
var string = t(literal,);
var number = t(literal,);
var content = t();
var heading = t(content,);
var keyword = t();
var operator = t();
var punctuation = t();
var bracket = t(punctuation,);
var meta = t();
var tags = {
  /**
  A comment.
  */
  comment,
  /**
  A line [comment](#highlight.tags.comment).
  */
  lineComment: t(comment,),
  /**
  A block [comment](#highlight.tags.comment).
  */
  blockComment: t(comment,),
  /**
  A documentation [comment](#highlight.tags.comment).
  */
  docComment: t(comment,),
  /**
  Any kind of identifier.
  */
  name,
  /**
  The [name](#highlight.tags.name) of a variable.
  */
  variableName: t(name,),
  /**
  A type [name](#highlight.tags.name).
  */
  typeName,
  /**
  A tag name (subtag of [`typeName`](#highlight.tags.typeName)).
  */
  tagName: t(typeName,),
  /**
  A property or field [name](#highlight.tags.name).
  */
  propertyName,
  /**
  An attribute name (subtag of [`propertyName`](#highlight.tags.propertyName)).
  */
  attributeName: t(propertyName,),
  /**
  The [name](#highlight.tags.name) of a class.
  */
  className: t(name,),
  /**
  A label [name](#highlight.tags.name).
  */
  labelName: t(name,),
  /**
  A namespace [name](#highlight.tags.name).
  */
  namespace: t(name,),
  /**
  The [name](#highlight.tags.name) of a macro.
  */
  macroName: t(name,),
  /**
  A literal value.
  */
  literal,
  /**
  A string [literal](#highlight.tags.literal).
  */
  string,
  /**
  A documentation [string](#highlight.tags.string).
  */
  docString: t(string,),
  /**
  A character literal (subtag of [string](#highlight.tags.string)).
  */
  character: t(string,),
  /**
  An attribute value (subtag of [string](#highlight.tags.string)).
  */
  attributeValue: t(string,),
  /**
  A number [literal](#highlight.tags.literal).
  */
  number,
  /**
  An integer [number](#highlight.tags.number) literal.
  */
  integer: t(number,),
  /**
  A floating-point [number](#highlight.tags.number) literal.
  */
  float: t(number,),
  /**
  A boolean [literal](#highlight.tags.literal).
  */
  bool: t(literal,),
  /**
  Regular expression [literal](#highlight.tags.literal).
  */
  regexp: t(literal,),
  /**
  An escape [literal](#highlight.tags.literal), for example a
  backslash escape in a string.
  */
  escape: t(literal,),
  /**
  A color [literal](#highlight.tags.literal).
  */
  color: t(literal,),
  /**
  A URL [literal](#highlight.tags.literal).
  */
  url: t(literal,),
  /**
  A language keyword.
  */
  keyword,
  /**
  The [keyword](#highlight.tags.keyword) for the self or this
  object.
  */
  self: t(keyword,),
  /**
  The [keyword](#highlight.tags.keyword) for null.
  */
  null: t(keyword,),
  /**
  A [keyword](#highlight.tags.keyword) denoting some atomic value.
  */
  atom: t(keyword,),
  /**
  A [keyword](#highlight.tags.keyword) that represents a unit.
  */
  unit: t(keyword,),
  /**
  A modifier [keyword](#highlight.tags.keyword).
  */
  modifier: t(keyword,),
  /**
  A [keyword](#highlight.tags.keyword) that acts as an operator.
  */
  operatorKeyword: t(keyword,),
  /**
  A control-flow related [keyword](#highlight.tags.keyword).
  */
  controlKeyword: t(keyword,),
  /**
  A [keyword](#highlight.tags.keyword) that defines something.
  */
  definitionKeyword: t(keyword,),
  /**
  A [keyword](#highlight.tags.keyword) related to defining or
  interfacing with modules.
  */
  moduleKeyword: t(keyword,),
  /**
  An operator.
  */
  operator,
  /**
  An [operator](#highlight.tags.operator) that dereferences something.
  */
  derefOperator: t(operator,),
  /**
  Arithmetic-related [operator](#highlight.tags.operator).
  */
  arithmeticOperator: t(operator,),
  /**
  Logical [operator](#highlight.tags.operator).
  */
  logicOperator: t(operator,),
  /**
  Bit [operator](#highlight.tags.operator).
  */
  bitwiseOperator: t(operator,),
  /**
  Comparison [operator](#highlight.tags.operator).
  */
  compareOperator: t(operator,),
  /**
  [Operator](#highlight.tags.operator) that updates its operand.
  */
  updateOperator: t(operator,),
  /**
  [Operator](#highlight.tags.operator) that defines something.
  */
  definitionOperator: t(operator,),
  /**
  Type-related [operator](#highlight.tags.operator).
  */
  typeOperator: t(operator,),
  /**
  Control-flow [operator](#highlight.tags.operator).
  */
  controlOperator: t(operator,),
  /**
  Program or markup punctuation.
  */
  punctuation,
  /**
  [Punctuation](#highlight.tags.punctuation) that separates
  things.
  */
  separator: t(punctuation,),
  /**
  Bracket-style [punctuation](#highlight.tags.punctuation).
  */
  bracket,
  /**
  Angle [brackets](#highlight.tags.bracket) (usually `<` and `>`
  tokens).
  */
  angleBracket: t(bracket,),
  /**
  Square [brackets](#highlight.tags.bracket) (usually `[` and `]`
  tokens).
  */
  squareBracket: t(bracket,),
  /**
  Parentheses (usually `(` and `)` tokens). Subtag of
  [bracket](#highlight.tags.bracket).
  */
  paren: t(bracket,),
  /**
  Braces (usually `{` and `}` tokens). Subtag of
  [bracket](#highlight.tags.bracket).
  */
  brace: t(bracket,),
  /**
  Content, for example plain text in XML or markup documents.
  */
  content,
  /**
  [Content](#highlight.tags.content) that represents a heading.
  */
  heading,
  /**
  A level 1 [heading](#highlight.tags.heading).
  */
  heading1: t(heading,),
  /**
  A level 2 [heading](#highlight.tags.heading).
  */
  heading2: t(heading,),
  /**
  A level 3 [heading](#highlight.tags.heading).
  */
  heading3: t(heading,),
  /**
  A level 4 [heading](#highlight.tags.heading).
  */
  heading4: t(heading,),
  /**
  A level 5 [heading](#highlight.tags.heading).
  */
  heading5: t(heading,),
  /**
  A level 6 [heading](#highlight.tags.heading).
  */
  heading6: t(heading,),
  /**
  A prose separator (such as a horizontal rule).
  */
  contentSeparator: t(content,),
  /**
  [Content](#highlight.tags.content) that represents a list.
  */
  list: t(content,),
  /**
  [Content](#highlight.tags.content) that represents a quote.
  */
  quote: t(content,),
  /**
  [Content](#highlight.tags.content) that is emphasized.
  */
  emphasis: t(content,),
  /**
  [Content](#highlight.tags.content) that is styled strong.
  */
  strong: t(content,),
  /**
  [Content](#highlight.tags.content) that is part of a link.
  */
  link: t(content,),
  /**
  [Content](#highlight.tags.content) that is styled as code or
  monospace.
  */
  monospace: t(content,),
  /**
  [Content](#highlight.tags.content) that has a strike-through
  style.
  */
  strikethrough: t(content,),
  /**
  Inserted text in a change-tracking format.
  */
  inserted: t(),
  /**
  Deleted text.
  */
  deleted: t(),
  /**
  Changed text.
  */
  changed: t(),
  /**
  An invalid or unsyntactic element.
  */
  invalid: t(),
  /**
  Metadata or meta-instruction.
  */
  meta,
  /**
  [Metadata](#highlight.tags.meta) that applies to the entire
  document.
  */
  documentMeta: t(meta,),
  /**
  [Metadata](#highlight.tags.meta) that annotates or adds
  attributes to a given syntactic element.
  */
  annotation: t(meta,),
  /**
  Processing instruction or preprocessor directive. Subtag of
  [meta](#highlight.tags.meta).
  */
  processingInstruction: t(meta,),
  /**
  [Modifier](#highlight.Tag^defineModifier) that indicates that a
  given element is being defined. Expected to be used with the
  various [name](#highlight.tags.name) tags.
  */
  definition: Tag.defineModifier(),
  /**
  [Modifier](#highlight.Tag^defineModifier) that indicates that
  something is constant. Mostly expected to be used with
  [variable names](#highlight.tags.variableName).
  */
  constant: Tag.defineModifier(),
  /**
  [Modifier](#highlight.Tag^defineModifier) used to indicate that
  a [variable](#highlight.tags.variableName) or [property
  name](#highlight.tags.propertyName) is being called or defined
  as a function.
  */
  function: Tag.defineModifier(),
  /**
  [Modifier](#highlight.Tag^defineModifier) that can be applied to
  [names](#highlight.tags.name) to indicate that they belong to
  the language's standard environment.
  */
  standard: Tag.defineModifier(),
  /**
  [Modifier](#highlight.Tag^defineModifier) that indicates a given
  [names](#highlight.tags.name) is local to some scope.
  */
  local: Tag.defineModifier(),
  /**
  A generic variant [modifier](#highlight.Tag^defineModifier) that
  can be used to tag language-specific alternative variants of
  some common tag. It is recommended for themes to define special
  forms of at least the [string](#highlight.tags.string) and
  [variable name](#highlight.tags.variableName) tags, since those
  come up a lot.
  */
  special: Tag.defineModifier(),
};
var classHighlighter = tagHighlighter([
  { tag: tags.link, class: 'tok-link', },
  { tag: tags.heading, class: 'tok-heading', },
  { tag: tags.emphasis, class: 'tok-emphasis', },
  { tag: tags.strong, class: 'tok-strong', },
  { tag: tags.keyword, class: 'tok-keyword', },
  { tag: tags.atom, class: 'tok-atom', },
  { tag: tags.bool, class: 'tok-bool', },
  { tag: tags.url, class: 'tok-url', },
  { tag: tags.labelName, class: 'tok-labelName', },
  { tag: tags.inserted, class: 'tok-inserted', },
  { tag: tags.deleted, class: 'tok-deleted', },
  { tag: tags.literal, class: 'tok-literal', },
  { tag: tags.string, class: 'tok-string', },
  { tag: tags.number, class: 'tok-number', },
  { tag: [tags.regexp, tags.escape, tags.special(tags.string,),], class: 'tok-string2', },
  { tag: tags.variableName, class: 'tok-variableName', },
  { tag: tags.local(tags.variableName,), class: 'tok-variableName tok-local', },
  { tag: tags.definition(tags.variableName,), class: 'tok-variableName tok-definition', },
  { tag: tags.special(tags.variableName,), class: 'tok-variableName2', },
  { tag: tags.definition(tags.propertyName,), class: 'tok-propertyName tok-definition', },
  { tag: tags.typeName, class: 'tok-typeName', },
  { tag: tags.namespace, class: 'tok-namespace', },
  { tag: tags.className, class: 'tok-className', },
  { tag: tags.macroName, class: 'tok-macroName', },
  { tag: tags.propertyName, class: 'tok-propertyName', },
  { tag: tags.operator, class: 'tok-operator', },
  { tag: tags.comment, class: 'tok-comment', },
  { tag: tags.meta, class: 'tok-meta', },
  { tag: tags.invalid, class: 'tok-invalid', },
  { tag: tags.punctuation, class: 'tok-punctuation', },
],);

// https :https://framerusercontent.com/modules/aJsTBlWNkIaM900KNIXT/TeoF57xeGMuVMjiI2ooR/codemirror_language.js
var C2 = '\u037C';
var COUNT2 = typeof Symbol == 'undefined' ? '__' + C2 : Symbol.for(C2,);
var SET2 = typeof Symbol == 'undefined' ? '__styleSet' + Math.floor(Math.random() * 1e8,) : Symbol('styleSet',);
var top2 = typeof globalThis != 'undefined' ? globalThis : typeof window != 'undefined' ? window : {};
var StyleModule2 = class {
  // :: () → string
  // Returns a string containing the module's CSS rules.
  getRules() {
    return this.rules.join('\n',);
  }
  // :: () → string
  // Generate a new unique CSS class name.
  static newName() {
    let id2 = top2[COUNT2] || 1;
    top2[COUNT2] = id2 + 1;
    return C2 + id2.toString(36,);
  }
  // :: (union<Document, ShadowRoot>, union<[StyleModule], StyleModule>)
  //
  // Mount the given set of modules in the given DOM root, which ensures
  // that the CSS rules defined by the module are available in that
  // context.
  //
  // Rules are only added to the document once per root.
  //
  // Rule order will follow the order of the modules, so that rules from
  // modules later in the array take precedence of those from earlier
  // modules. If you call this function multiple times for the same root
  // in a way that changes the order of already mounted modules, the old
  // order will be changed.
  static mount(root, modules,) {
    (root[SET2] || new StyleSet2(root,)).mount(Array.isArray(modules,) ? modules : [modules,],);
  }
  // :: (Object<Style>, ?{finish: ?(string) → string})
  // Create a style module from the given spec.
  //
  // When `finish` is given, it is called on regular (non-`@`)
  // selectors (after `&` expansion) to compute the final selector.
  constructor(spec, options,) {
    this.rules = [];
    let { finish, } = options || {};
    function splitSelector(selector,) {
      return /^@/.test(selector,) ? [selector,] : selector.split(/,\s*/,);
    }
    function render(selectors, spec2, target, isKeyframes,) {
      let local = [], isAt = /^@(\w+)\b/.exec(selectors[0],), keyframes = isAt && isAt[1] == 'keyframes';
      if (isAt && spec2 == null) return target.push(selectors[0] + ';',);
      for (let prop in spec2) {
        let value = spec2[prop];
        if (/&/.test(prop,)) {
          render(
            prop.split(/,\s*/,).map((part,) => selectors.map((sel,) => part.replace(/&/, sel,))).reduce((a, b,) => a.concat(b,)),
            value,
            target,
          );
        } else if (value && typeof value == 'object') {
          if (!isAt) throw new RangeError('The value of a property (' + prop + ') should be a primitive value.',);
          render(splitSelector(prop,), value, local, keyframes,);
        } else if (value != null) {
          local.push(prop.replace(/_.*/, '',).replace(/[A-Z]/g, (l,) => '-' + l.toLowerCase(),) + ': ' + value + ';',);
        }
      }
      if (local.length || keyframes) {
        target.push((finish && !isAt && !isKeyframes ? selectors.map(finish,) : selectors).join(', ',) + ' {' + local.join(' ',) + '}',);
      }
    }
    for (let prop in spec) render(splitSelector(prop,), spec[prop], this.rules,);
  }
};
var adoptedSet2 = /* @__PURE__ */ new Map();
var StyleSet2 = class {
  mount(modules,) {
    let sheet = this.sheet;
    let pos = 0, j = 0;
    for (let i2 = 0; i2 < modules.length; i2++) {
      let mod = modules[i2], index = this.modules.indexOf(mod,);
      if (index < j && index > -1) {
        this.modules.splice(index, 1,);
        j--;
        index = -1;
      }
      if (index == -1) {
        this.modules.splice(j++, 0, mod,);
        if (sheet) for (let k = 0; k < mod.rules.length; k++) sheet.insertRule(mod.rules[k], pos++,);
      } else {
        while (j < index) pos += this.modules[j++].rules.length;
        pos += mod.rules.length;
        j++;
      }
    }
    if (!sheet) {
      let text = '';
      for (let i1 = 0; i1 < this.modules.length; i1++) text += this.modules[i1].getRules() + '\n';
      this.styleTag.textContent = text;
    }
  }
  constructor(root,) {
    let doc2 = root.ownerDocument || root, win = doc2.defaultView;
    if (!root.head && root.adoptedStyleSheets && win.CSSStyleSheet) {
      let adopted = adoptedSet2.get(doc2,);
      if (adopted) {
        root.adoptedStyleSheets = [adopted.sheet, ...root.adoptedStyleSheets,];
        return root[SET2] = adopted;
      }
      this.sheet = new win.CSSStyleSheet();
      root.adoptedStyleSheets = [this.sheet, ...root.adoptedStyleSheets,];
      adoptedSet2.set(doc2, this,);
    } else {
      this.styleTag = doc2.createElement('style',);
      let target = root.head || root;
      target.insertBefore(this.styleTag, target.firstChild,);
    }
    this.modules = [];
    root[SET2] = this;
  }
};
var _a;
var languageDataProp = /* @__PURE__ */ new NodeProp();
function defineLanguageFacet(baseData,) {
  return Facet.define({ combine: baseData ? (values,) => values.concat(baseData,) : void 0, },);
}
var sublanguageProp = /* @__PURE__ */ new NodeProp();
var Language = class {
  /**
  Query whether this language is active at the given position.
  */
  isActiveAt(state, pos, side = -1,) {
    return topNodeAt(state, pos, side,).type.prop(languageDataProp,) == this.data;
  }
  /**
  Find the document regions that were parsed using this language.
  The returned regions will _include_ any nested languages rooted
  in this language, when those exist.
  */
  findRegions(state,) {
    let lang = state.facet(language,);
    if ((lang === null || lang === void 0 ? void 0 : lang.data) == this.data) return [{ from: 0, to: state.doc.length, },];
    if (!lang || !lang.allowsNesting) return [];
    let result = [];
    let explore = (tree, from,) => {
      if (tree.prop(languageDataProp,) == this.data) {
        result.push({ from, to: from + tree.length, },);
        return;
      }
      let mount = tree.prop(NodeProp.mounted,);
      if (mount) {
        if (mount.tree.prop(languageDataProp,) == this.data) {
          if (mount.overlay) for (let r of mount.overlay) result.push({ from: r.from + from, to: r.to + from, },);
          else result.push({ from, to: from + tree.length, },);
          return;
        } else if (mount.overlay) {
          let size = result.length;
          explore(mount.tree, mount.overlay[0].from + from,);
          if (result.length > size) return;
        }
      }
      for (let i2 = 0; i2 < tree.children.length; i2++) {
        let ch = tree.children[i2];
        if (ch instanceof Tree) explore(ch, tree.positions[i2] + from,);
      }
    };
    explore(syntaxTree(state,), 0,);
    return result;
  }
  /**
  Indicates whether this language allows nested languages. The
  default implementation returns true.
  */
  get allowsNesting() {
    return true;
  }
  /**
  Construct a language object. If you need to invoke this
  directly, first define a data facet with
  [`defineLanguageFacet`](https://codemirror.net/6/docs/ref/#language.defineLanguageFacet), and then
  configure your parser to [attach](https://codemirror.net/6/docs/ref/#language.languageDataProp) it
  to the language's outer syntax node.
  */
  constructor(data, parser, extraExtensions = [], name2 = '',) {
    this.data = data;
    this.name = name2;
    if (!EditorState.prototype.hasOwnProperty('tree',)) {
      Object.defineProperty(EditorState.prototype, 'tree', {
        get() {
          return syntaxTree(this,);
        },
      },);
    }
    this.parser = parser;
    this.extension = [
      language.of(this,),
      EditorState.languageData.of((state, pos, side,) => {
        let top22 = topNodeAt(state, pos, side,), data2 = top22.type.prop(languageDataProp,);
        if (!data2) return [];
        let base2 = state.facet(data2,), sub = top22.type.prop(sublanguageProp,);
        if (sub) {
          let innerNode = top22.resolve(pos - top22.from, side,);
          for (let sublang of sub) {
            if (sublang.test(innerNode, state,)) {
              let data3 = state.facet(sublang.facet,);
              return sublang.type == 'replace' ? data3 : data3.concat(base2,);
            }
          }
        }
        return base2;
      },),
    ].concat(extraExtensions,);
  }
};
Language.setState = /* @__PURE__ */ StateEffect.define();
function topNodeAt(state, pos, side,) {
  let topLang = state.facet(language,), tree = syntaxTree(state,).topNode;
  if (!topLang || topLang.allowsNesting) {
    for (let node = tree; node; node = node.enter(pos, side, IterMode.ExcludeBuffers,)) if (node.type.isTop) tree = node;
  }
  return tree;
}
var LRLanguage = class extends Language {
  /**
  Define a language from a parser.
  */
  static define(spec,) {
    let data = defineLanguageFacet(spec.languageData,);
    return new LRLanguage(
      data,
      spec.parser.configure({ props: [languageDataProp.add((type,) => type.isTop ? data : void 0),], },),
      spec.name,
    );
  }
  /**
  Create a new instance of this language with a reconfigured
  version of its parser and optionally a new name.
  */
  configure(options, name2,) {
    return new LRLanguage(this.data, this.parser.configure(options,), name2 || this.name,);
  }
  get allowsNesting() {
    return this.parser.hasWrappers();
  }
  constructor(data, parser, name2,) {
    super(data, parser, [], name2,);
    this.parser = parser;
  }
};
function syntaxTree(state,) {
  let field = state.field(Language.state, false,);
  return field ? field.tree : Tree.empty;
}
var DocInput = class {
  get length() {
    return this.doc.length;
  }
  syncTo(pos,) {
    this.string = this.cursor.next(pos - this.cursorPos,).value;
    this.cursorPos = pos + this.string.length;
    return this.cursorPos - this.string.length;
  }
  chunk(pos,) {
    this.syncTo(pos,);
    return this.string;
  }
  get lineChunks() {
    return true;
  }
  read(from, to,) {
    let stringStart = this.cursorPos - this.string.length;
    if (from < stringStart || to >= this.cursorPos) return this.doc.sliceString(from, to,);
    else return this.string.slice(from - stringStart, to - stringStart,);
  }
  /**
  Create an input object for the given document.
  */
  constructor(doc2,) {
    this.doc = doc2;
    this.cursorPos = 0;
    this.string = '';
    this.cursor = doc2.iter();
  }
};
var currentContext = null;
var ParseContext = class {
  /**
  @internal
  */
  static create(parser, state, viewport,) {
    return new ParseContext(parser, state, [], Tree.empty, 0, viewport, [], null,);
  }
  startParse() {
    return this.parser.startParse(new DocInput(this.state.doc,), this.fragments,);
  }
  /**
  @internal
  */
  work(until, upto,) {
    if (upto != null && upto >= this.state.doc.length) upto = void 0;
    if (this.tree != Tree.empty && this.isDone(upto !== null && upto !== void 0 ? upto : this.state.doc.length,)) {
      this.takeTree();
      return true;
    }
    return this.withContext(() => {
      var _a2;
      if (typeof until == 'number') {
        let endTime = Date.now() + until;
        until = () => Date.now() > endTime;
      }
      if (!this.parse) this.parse = this.startParse();
      if (upto != null && (this.parse.stoppedAt == null || this.parse.stoppedAt > upto) && upto < this.state.doc.length) {
        this.parse.stopAt(upto,);
      }
      for (;;) {
        let done = this.parse.advance();
        if (done) {
          this.fragments = this.withoutTempSkipped(TreeFragment.addTree(done, this.fragments, this.parse.stoppedAt != null,),);
          this.treeLen = (_a2 = this.parse.stoppedAt) !== null && _a2 !== void 0 ? _a2 : this.state.doc.length;
          this.tree = done;
          this.parse = null;
          if (this.treeLen < (upto !== null && upto !== void 0 ? upto : this.state.doc.length)) this.parse = this.startParse();
          else return true;
        }
        if (until()) return false;
      }
    },);
  }
  /**
  @internal
  */
  takeTree() {
    let pos, tree;
    if (this.parse && (pos = this.parse.parsedPos) >= this.treeLen) {
      if (this.parse.stoppedAt == null || this.parse.stoppedAt > pos) this.parse.stopAt(pos,);
      this.withContext(() => {
        while (!(tree = this.parse.advance())) {
        }
      },);
      this.treeLen = pos;
      this.tree = tree;
      this.fragments = this.withoutTempSkipped(TreeFragment.addTree(this.tree, this.fragments, true,),);
      this.parse = null;
    }
  }
  withContext(f,) {
    let prev = currentContext;
    currentContext = this;
    try {
      return f();
    } finally {
      currentContext = prev;
    }
  }
  withoutTempSkipped(fragments,) {
    for (let r; r = this.tempSkipped.pop();) fragments = cutFragments(fragments, r.from, r.to,);
    return fragments;
  }
  /**
  @internal
  */
  changes(changes, newState,) {
    let { fragments, tree, treeLen, viewport, skipped, } = this;
    this.takeTree();
    if (!changes.empty) {
      let ranges = [];
      changes.iterChangedRanges((fromA, toA, fromB, toB,) => ranges.push({ fromA, toA, fromB, toB, },));
      fragments = TreeFragment.applyChanges(fragments, ranges,);
      tree = Tree.empty;
      treeLen = 0;
      viewport = { from: changes.mapPos(viewport.from, -1,), to: changes.mapPos(viewport.to, 1,), };
      if (this.skipped.length) {
        skipped = [];
        for (let r of this.skipped) {
          let from = changes.mapPos(r.from, 1,), to = changes.mapPos(r.to, -1,);
          if (from < to) skipped.push({ from, to, },);
        }
      }
    }
    return new ParseContext(this.parser, newState, fragments, tree, treeLen, viewport, skipped, this.scheduleOn,);
  }
  /**
  @internal
  */
  updateViewport(viewport,) {
    if (this.viewport.from == viewport.from && this.viewport.to == viewport.to) return false;
    this.viewport = viewport;
    let startLen = this.skipped.length;
    for (let i2 = 0; i2 < this.skipped.length; i2++) {
      let { from, to, } = this.skipped[i2];
      if (from < viewport.to && to > viewport.from) {
        this.fragments = cutFragments(this.fragments, from, to,);
        this.skipped.splice(i2--, 1,);
      }
    }
    if (this.skipped.length >= startLen) return false;
    this.reset();
    return true;
  }
  /**
  @internal
  */
  reset() {
    if (this.parse) {
      this.takeTree();
      this.parse = null;
    }
  }
  /**
  Notify the parse scheduler that the given region was skipped
  because it wasn't in view, and the parse should be restarted
  when it comes into view.
  */
  skipUntilInView(from, to,) {
    this.skipped.push({ from, to, },);
  }
  /**
  Returns a parser intended to be used as placeholder when
  asynchronously loading a nested parser. It'll skip its input and
  mark it as not-really-parsed, so that the next update will parse
  it again.

  When `until` is given, a reparse will be scheduled when that
  promise resolves.
  */
  static getSkippingParser(until,) {
    return new class extends Parser {
      createParse(input, fragments, ranges,) {
        let from = ranges[0].from, to = ranges[ranges.length - 1].to;
        let parser = {
          parsedPos: from,
          advance() {
            let cx = currentContext;
            if (cx) {
              for (let r of ranges) cx.tempSkipped.push(r,);
              if (until) cx.scheduleOn = cx.scheduleOn ? Promise.all([cx.scheduleOn, until,],) : until;
            }
            this.parsedPos = to;
            return new Tree(NodeType.none, [], [], to - from,);
          },
          stoppedAt: null,
          stopAt() {
          },
        };
        return parser;
      }
    }();
  }
  /**
  @internal
  */
  isDone(upto,) {
    upto = Math.min(upto, this.state.doc.length,);
    let frags = this.fragments;
    return this.treeLen >= upto && frags.length && frags[0].from == 0 && frags[0].to >= upto;
  }
  /**
  Get the context for the current parse, or `null` if no editor
  parse is in progress.
  */
  static get() {
    return currentContext;
  }
  constructor(parser, state, fragments = [], tree, treeLen, viewport, skipped, scheduleOn,) {
    this.parser = parser;
    this.state = state;
    this.fragments = fragments;
    this.tree = tree;
    this.treeLen = treeLen;
    this.viewport = viewport;
    this.skipped = skipped;
    this.scheduleOn = scheduleOn;
    this.parse = null;
    this.tempSkipped = [];
  }
};
function cutFragments(fragments, from, to,) {
  return TreeFragment.applyChanges(fragments, [{ fromA: from, toA: to, fromB: from, toB: to, },],);
}
var LanguageState = class {
  apply(tr,) {
    if (!tr.docChanged && this.tree == this.context.tree) return this;
    let newCx = this.context.changes(tr.changes, tr.state,);
    let upto = this.context.treeLen == tr.startState.doc.length
      ? void 0
      : Math.max(tr.changes.mapPos(this.context.treeLen,), newCx.viewport.to,);
    if (!newCx.work(20, upto,)) newCx.takeTree();
    return new LanguageState(newCx,);
  }
  static init(state,) {
    let vpTo = Math.min(3e3, state.doc.length,);
    let parseState = ParseContext.create(state.facet(language,).parser, state, { from: 0, to: vpTo, },);
    if (!parseState.work(20, vpTo,)) parseState.takeTree();
    return new LanguageState(parseState,);
  }
  constructor(context,) {
    this.context = context;
    this.tree = context.tree;
  }
};
Language.state = /* @__PURE__ */ StateField.define({
  create: LanguageState.init,
  update(value, tr,) {
    for (let e of tr.effects) if (e.is(Language.setState,)) return e.value;
    if (tr.startState.facet(language,) != tr.state.facet(language,)) return LanguageState.init(tr.state,);
    return value.apply(tr,);
  },
},);
var requestIdle = (callback,) => {
  let timeout = setTimeout(() => callback(), 500,);
  return () => clearTimeout(timeout,);
};
if (typeof requestIdleCallback != 'undefined') {
  requestIdle = (callback,) => {
    let idle = -1,
      timeout = setTimeout(() => {
        idle = requestIdleCallback(callback, { timeout: 500 - 100, },);
      }, 100,);
    return () => idle < 0 ? clearTimeout(timeout,) : cancelIdleCallback(idle,);
  };
}
var isInputPending = typeof navigator != 'undefined' && ((_a = navigator.scheduling) === null || _a === void 0 ? void 0 : _a.isInputPending)
  ? () => navigator.scheduling.isInputPending()
  : null;
var parseWorker = /* @__PURE__ */ ViewPlugin.fromClass(
  class ParseWorker {
    update(update,) {
      let cx = this.view.state.field(Language.state,).context;
      if (cx.updateViewport(update.view.viewport,) || this.view.viewport.to > cx.treeLen) this.scheduleWork();
      if (update.docChanged) {
        if (this.view.hasFocus) this.chunkBudget += 50;
        this.scheduleWork();
      }
      this.checkAsyncSchedule(cx,);
    }
    scheduleWork() {
      if (this.working) return;
      let { state, } = this.view, field = state.field(Language.state,);
      if (field.tree != field.context.tree || !field.context.isDone(state.doc.length,)) this.working = requestIdle(this.work,);
    }
    work(deadline,) {
      this.working = null;
      let now = Date.now();
      if (this.chunkEnd < now && (this.chunkEnd < 0 || this.view.hasFocus)) {
        this.chunkEnd = now + 3e4;
        this.chunkBudget = 3e3;
      }
      if (this.chunkBudget <= 0) return;
      let { state, viewport: { to: vpTo, }, } = this.view, field = state.field(Language.state,);
      if (field.tree == field.context.tree && field.context.isDone(vpTo + 1e5,)) return;
      let endTime = Date.now() +
        Math.min(this.chunkBudget, 100, deadline && !isInputPending ? Math.max(25, deadline.timeRemaining() - 5,) : 1e9,);
      let viewportFirst = field.context.treeLen < vpTo && state.doc.length > vpTo + 1e3;
      let done = field.context.work(() => {
        return isInputPending && isInputPending() || Date.now() > endTime;
      }, vpTo + (viewportFirst ? 0 : 1e5),);
      this.chunkBudget -= Date.now() - now;
      if (done || this.chunkBudget <= 0) {
        field.context.takeTree();
        this.view.dispatch({ effects: Language.setState.of(new LanguageState(field.context,),), },);
      }
      if (this.chunkBudget > 0 && !(done && !viewportFirst)) this.scheduleWork();
      this.checkAsyncSchedule(field.context,);
    }
    checkAsyncSchedule(cx,) {
      if (cx.scheduleOn) {
        this.workScheduled++;
        cx.scheduleOn.then(() => this.scheduleWork()).catch((err,) => logException(this.view.state, err,)).then(() => this.workScheduled--);
        cx.scheduleOn = null;
      }
    }
    destroy() {
      if (this.working) this.working();
    }
    isWorking() {
      return !!(this.working || this.workScheduled > 0);
    }
    constructor(view,) {
      this.view = view;
      this.working = null;
      this.workScheduled = 0;
      this.chunkEnd = -1;
      this.chunkBudget = -1;
      this.work = this.work.bind(this,);
      this.scheduleWork();
    }
  },
  {
    eventHandlers: {
      focus() {
        this.scheduleWork();
      },
    },
  },
);
var language = /* @__PURE__ */ Facet.define({
  combine(languages,) {
    return languages.length ? languages[0] : null;
  },
  enables: (language2,) => [
    Language.state,
    parseWorker,
    EditorView.contentAttributes.compute([language2,], (state,) => {
      let lang = state.facet(language2,);
      return lang && lang.name ? { 'data-language': lang.name, } : {};
    },),
  ],
},);
var LanguageSupport = class {
  /**
  Create a language support object.
  */
  constructor(language2, support = [],) {
    this.language = language2;
    this.support = support;
    this.extension = [language2, support,];
  }
};
var LanguageDescription = class {
  /**
  Start loading the the language. Will return a promise that
  resolves to a [`LanguageSupport`](https://codemirror.net/6/docs/ref/#language.LanguageSupport)
  object when the language successfully loads.
  */
  load() {
    return this.loading || (this.loading = this.loadFunc().then((support,) => this.support = support, (err,) => {
      this.loading = null;
      throw err;
    },));
  }
  /**
  Create a language description.
  */
  static of(spec,) {
    let { load, support, } = spec;
    if (!load) {
      if (!support) throw new RangeError('Must pass either \'load\' or \'support\' to LanguageDescription.of',);
      load = () => Promise.resolve(support,);
    }
    return new LanguageDescription(
      spec.name,
      (spec.alias || []).concat(spec.name,).map((s,) => s.toLowerCase()),
      spec.extensions || [],
      spec.filename,
      load,
      support,
    );
  }
  /**
  Look for a language in the given array of descriptions that
  matches the filename. Will first match
  [`filename`](https://codemirror.net/6/docs/ref/#language.LanguageDescription.filename) patterns,
  and then [extensions](https://codemirror.net/6/docs/ref/#language.LanguageDescription.extensions),
  and return the first language that matches.
  */
  static matchFilename(descs, filename,) {
    for (let d of descs) if (d.filename && d.filename.test(filename,)) return d;
    let ext = /\.([^.]+)$/.exec(filename,);
    if (ext) {
      for (let d1 of descs) if (d1.extensions.indexOf(ext[1],) > -1) return d1;
    }
    return null;
  }
  /**
  Look for a language whose name or alias matches the the given
  name (case-insensitively). If `fuzzy` is true, and no direct
  matchs is found, this'll also search for a language whose name
  or alias occurs in the string (for names shorter than three
  characters, only when surrounded by non-word characters).
  */
  static matchLanguageName(descs, name2, fuzzy = true,) {
    name2 = name2.toLowerCase();
    for (let d of descs) if (d.alias.some((a,) => a == name2)) return d;
    if (fuzzy) {
      for (let d1 of descs) {
        for (let a of d1.alias) {
          let found = name2.indexOf(a,);
          if (found > -1 && (a.length > 2 || !/\w/.test(name2[found - 1],) && !/\w/.test(name2[found + a.length],))) return d1;
        }
      }
    }
    return null;
  }
  constructor(name2, alias, extensions, filename, loadFunc, support = void 0,) {
    this.name = name2;
    this.alias = alias;
    this.extensions = extensions;
    this.filename = filename;
    this.loadFunc = loadFunc;
    this.support = support;
    this.loading = null;
  }
};
var indentService = /* @__PURE__ */ Facet.define();
var indentUnit = /* @__PURE__ */ Facet.define({
  combine: (values,) => {
    if (!values.length) return '  ';
    let unit = values[0];
    if (!unit || /\S/.test(unit,) || Array.from(unit,).some((e,) => e != unit[0])) {
      throw new Error('Invalid indent unit: ' + JSON.stringify(values[0],),);
    }
    return unit;
  },
},);
function getIndentUnit(state,) {
  let unit = state.facet(indentUnit,);
  return unit.charCodeAt(0,) == 9 ? state.tabSize * unit.length : unit.length;
}
function indentString(state, cols,) {
  let result = '', ts = state.tabSize, ch = state.facet(indentUnit,)[0];
  if (ch == '	') {
    while (cols >= ts) {
      result += '	';
      cols -= ts;
    }
    ch = ' ';
  }
  for (let i2 = 0; i2 < cols; i2++) result += ch;
  return result;
}
function getIndentation(context, pos,) {
  if (context instanceof EditorState) context = new IndentContext(context,);
  for (let service of context.state.facet(indentService,)) {
    let result = service(context, pos,);
    if (result !== void 0) return result;
  }
  let tree = syntaxTree(context.state,);
  return tree ? syntaxIndentation(context, tree, pos,) : null;
}
var IndentContext = class {
  /**
  Get a description of the line at the given position, taking
  [simulated line
  breaks](https://codemirror.net/6/docs/ref/#language.IndentContext.constructor^options.simulateBreak)
  into account. If there is such a break at `pos`, the `bias`
  argument determines whether the part of the line line before or
  after the break is used.
  */
  lineAt(pos, bias = 1,) {
    let line = this.state.doc.lineAt(pos,);
    let { simulateBreak, simulateDoubleBreak, } = this.options;
    if (simulateBreak != null && simulateBreak >= line.from && simulateBreak <= line.to) {
      if (simulateDoubleBreak && simulateBreak == pos) return { text: '', from: pos, };
      else if (bias < 0 ? simulateBreak < pos : simulateBreak <= pos) {
        return { text: line.text.slice(simulateBreak - line.from,), from: simulateBreak, };
      } else return { text: line.text.slice(0, simulateBreak - line.from,), from: line.from, };
    }
    return line;
  }
  /**
  Get the text directly after `pos`, either the entire line
  or the next 100 characters, whichever is shorter.
  */
  textAfterPos(pos, bias = 1,) {
    if (this.options.simulateDoubleBreak && pos == this.options.simulateBreak) return '';
    let { text, from, } = this.lineAt(pos, bias,);
    return text.slice(pos - from, Math.min(text.length, pos + 100 - from,),);
  }
  /**
  Find the column for the given position.
  */
  column(pos, bias = 1,) {
    let { text, from, } = this.lineAt(pos, bias,);
    let result = this.countColumn(text, pos - from,);
    let override = this.options.overrideIndentation ? this.options.overrideIndentation(from,) : -1;
    if (override > -1) result += override - this.countColumn(text, text.search(/\S|$/,),);
    return result;
  }
  /**
  Find the column position (taking tabs into account) of the given
  position in the given string.
  */
  countColumn(line, pos = line.length,) {
    return countColumn(line, this.state.tabSize, pos,);
  }
  /**
  Find the indentation column of the line at the given point.
  */
  lineIndent(pos, bias = 1,) {
    let { text, from, } = this.lineAt(pos, bias,);
    let override = this.options.overrideIndentation;
    if (override) {
      let overriden = override(from,);
      if (overriden > -1) return overriden;
    }
    return this.countColumn(text, text.search(/\S|$/,),);
  }
  /**
  Returns the [simulated line
  break](https://codemirror.net/6/docs/ref/#language.IndentContext.constructor^options.simulateBreak)
  for this context, if any.
  */
  get simulatedBreak() {
    return this.options.simulateBreak || null;
  }
  /**
  Create an indent context.
  */
  constructor(state, options = {},) {
    this.state = state;
    this.options = options;
    this.unit = getIndentUnit(state,);
  }
};
var indentNodeProp = /* @__PURE__ */ new NodeProp();
function syntaxIndentation(cx, ast, pos,) {
  return indentFrom(ast.resolveInner(pos,).enterUnfinishedNodesBefore(pos,), pos, cx,);
}
function ignoreClosed(cx,) {
  return cx.pos == cx.options.simulateBreak && cx.options.simulateDoubleBreak;
}
function indentStrategy(tree,) {
  let strategy = tree.type.prop(indentNodeProp,);
  if (strategy) return strategy;
  let first = tree.firstChild, close;
  if (first && (close = first.type.prop(NodeProp.closedBy,))) {
    let last = tree.lastChild, closed = last && close.indexOf(last.name,) > -1;
    return (cx,) => delimitedStrategy(cx, true, 1, void 0, closed && !ignoreClosed(cx,) ? last.from : void 0,);
  }
  return tree.parent == null ? topIndent : null;
}
function indentFrom(node, pos, base2,) {
  for (; node; node = node.parent) {
    let strategy = indentStrategy(node,);
    if (strategy) return strategy(TreeIndentContext.create(base2, pos, node,),);
  }
  return null;
}
function topIndent() {
  return 0;
}
var TreeIndentContext = class extends IndentContext {
  /**
  @internal
  */
  static create(base2, pos, node,) {
    return new TreeIndentContext(base2, pos, node,);
  }
  /**
  Get the text directly after `this.pos`, either the entire line
  or the next 100 characters, whichever is shorter.
  */
  get textAfter() {
    return this.textAfterPos(this.pos,);
  }
  /**
  Get the indentation at the reference line for `this.node`, which
  is the line on which it starts, unless there is a node that is
  _not_ a parent of this node covering the start of that line. If
  so, the line at the start of that node is tried, again skipping
  on if it is covered by another such node.
  */
  get baseIndent() {
    let line = this.state.doc.lineAt(this.node.from,);
    for (;;) {
      let atBreak = this.node.resolve(line.from,);
      while (atBreak.parent && atBreak.parent.from == atBreak.from) atBreak = atBreak.parent;
      if (isParent(atBreak, this.node,)) break;
      line = this.state.doc.lineAt(atBreak.from,);
    }
    return this.lineIndent(line.from,);
  }
  /**
  Continue looking for indentations in the node's parent nodes,
  and return the result of that.
  */
  continue() {
    let parent = this.node.parent;
    return parent ? indentFrom(parent, this.pos, this.base,) : 0;
  }
  constructor(base2, pos, node,) {
    super(base2.state, base2.options,);
    this.base = base2;
    this.pos = pos;
    this.node = node;
  }
};
function isParent(parent, of,) {
  for (let cur = of; cur; cur = cur.parent) if (parent == cur) return true;
  return false;
}
function bracketedAligned(context,) {
  let tree = context.node;
  let openToken = tree.childAfter(tree.from,), last = tree.lastChild;
  if (!openToken) return null;
  let sim = context.options.simulateBreak;
  let openLine = context.state.doc.lineAt(openToken.from,);
  let lineEnd = sim == null || sim <= openLine.from ? openLine.to : Math.min(openLine.to, sim,);
  for (let pos = openToken.to;;) {
    let next = tree.childAfter(pos,);
    if (!next || next == last) return null;
    if (!next.type.isSkipped) return next.from < lineEnd ? openToken : null;
    pos = next.to;
  }
}
function delimitedIndent({ closing, align = true, units = 1, },) {
  return (context,) => delimitedStrategy(context, align, units, closing,);
}
function delimitedStrategy(context, align, units, closing, closedAt,) {
  let after = context.textAfter, space = after.match(/^\s*/,)[0].length;
  let closed = closing && after.slice(space, space + closing.length,) == closing || closedAt == context.pos + space;
  let aligned = align ? bracketedAligned(context,) : null;
  if (aligned) return closed ? context.column(aligned.from,) : context.column(aligned.to,);
  return context.baseIndent + (closed ? 0 : context.unit * units);
}
var flatIndent = (context,) => context.baseIndent;
function continuedIndent({ except, units = 1, } = {},) {
  return (context,) => {
    let matchExcept = except && except.test(context.textAfter,);
    return context.baseIndent + (matchExcept ? 0 : units * context.unit);
  };
}
var foldService = /* @__PURE__ */ Facet.define();
var foldNodeProp = /* @__PURE__ */ new NodeProp();
function foldInside(node,) {
  let first = node.firstChild, last = node.lastChild;
  return first && first.to < last.from ? { from: first.to, to: last.type.isError ? node.to : last.from, } : null;
}
var HighlightStyle = class {
  /**
  Create a highlighter style that associates the given styles to
  the given tags. The specs must be objects that hold a style tag
  or array of tags in their `tag` property, and either a single
  `class` property providing a static CSS class (for highlighter
  that rely on external styling), or a
  [`style-mod`](https://github.com/marijnh/style-mod#documentation)-style
  set of CSS properties (which define the styling for those tags).

  The CSS rules created for a highlighter will be emitted in the
  order of the spec's properties. That means that for elements that
  have multiple tags associated with them, styles defined further
  down in the list will have a higher CSS precedence than styles
  defined earlier.
  */
  static define(specs, options,) {
    return new HighlightStyle(specs, options || {},);
  }
  constructor(specs, options,) {
    this.specs = specs;
    let modSpec;
    function def(spec,) {
      let cls = StyleModule2.newName();
      (modSpec || (modSpec = /* @__PURE__ */ Object.create(null,)))['.' + cls] = spec;
      return cls;
    }
    const all = typeof options.all == 'string' ? options.all : options.all ? def(options.all,) : void 0;
    const scopeOpt = options.scope;
    this.scope = scopeOpt instanceof Language
      ? (type,) => type.prop(languageDataProp,) == scopeOpt.data
      : scopeOpt
      ? (type,) => type == scopeOpt
      : void 0;
    this.style =
      tagHighlighter(specs.map((style,) => ({ tag: style.tag, class: style.class || def(Object.assign({}, style, { tag: null, },),), })), {
        all,
      },).style;
    this.module = modSpec ? new StyleModule2(modSpec,) : null;
    this.themeType = options.themeType;
  }
};
var highlighterFacet = /* @__PURE__ */ Facet.define();
var fallbackHighlighter = /* @__PURE__ */ Facet.define({
  combine(values,) {
    return values.length ? [values[0],] : null;
  },
},);
function getHighlighters(state,) {
  let main = state.facet(highlighterFacet,);
  return main.length ? main : state.facet(fallbackHighlighter,);
}
function syntaxHighlighting(highlighter, options,) {
  let ext = [treeHighlighter,], themeType;
  if (highlighter instanceof HighlightStyle) {
    if (highlighter.module) ext.push(EditorView.styleModule.of(highlighter.module,),);
    themeType = highlighter.themeType;
  }
  if (options === null || options === void 0 ? void 0 : options.fallback) ext.push(fallbackHighlighter.of(highlighter,),);
  else if (themeType) {
    ext.push(highlighterFacet.computeN([EditorView.darkTheme,], (state,) => {
      return state.facet(EditorView.darkTheme,) == (themeType == 'dark') ? [highlighter,] : [];
    },),);
  } else ext.push(highlighterFacet.of(highlighter,),);
  return ext;
}
var TreeHighlighter = class {
  update(update,) {
    let tree = syntaxTree(update.state,), highlighters = getHighlighters(update.state,);
    let styleChange = highlighters != getHighlighters(update.startState,);
    if (tree.length < update.view.viewport.to && !styleChange && tree.type == this.tree.type) {
      this.decorations = this.decorations.map(update.changes,);
    } else if (tree != this.tree || update.viewportChanged || styleChange) {
      this.tree = tree;
      this.decorations = this.buildDeco(update.view, highlighters,);
    }
  }
  buildDeco(view, highlighters,) {
    if (!highlighters || !this.tree.length) return Decoration.none;
    let builder = new RangeSetBuilder();
    for (let { from, to, } of view.visibleRanges) {
      highlightTree(
        this.tree,
        highlighters,
        (from2, to2, style,) => {
          builder.add(from2, to2, this.markCache[style] || (this.markCache[style] = Decoration.mark({ class: style, },)),);
        },
        from,
        to,
      );
    }
    return builder.finish();
  }
  constructor(view,) {
    this.markCache = /* @__PURE__ */ Object.create(null,);
    this.tree = syntaxTree(view.state,);
    this.decorations = this.buildDeco(view, getHighlighters(view.state,),);
  }
};
var treeHighlighter = /* @__PURE__ */ Prec.high(
  /* @__PURE__ */ ViewPlugin.fromClass(TreeHighlighter, { decorations: (v,) => v.decorations, },),
);
var defaultHighlightStyle = /* @__PURE__ */ HighlightStyle.define([
  { tag: tags.meta, color: '#404740', },
  { tag: tags.link, textDecoration: 'underline', },
  { tag: tags.heading, textDecoration: 'underline', fontWeight: 'bold', },
  { tag: tags.emphasis, fontStyle: 'italic', },
  { tag: tags.strong, fontWeight: 'bold', },
  { tag: tags.strikethrough, textDecoration: 'line-through', },
  { tag: tags.keyword, color: '#708', },
  { tag: [tags.atom, tags.bool, tags.url, tags.contentSeparator, tags.labelName,], color: '#219', },
  { tag: [tags.literal, tags.inserted,], color: '#164', },
  { tag: [tags.string, tags.deleted,], color: '#a11', },
  { tag: [tags.regexp, tags.escape, /* @__PURE__ */ tags.special(tags.string,),], color: '#e40', },
  { tag: /* @__PURE__ */ tags.definition(tags.variableName,), color: '#00f', },
  { tag: /* @__PURE__ */ tags.local(tags.variableName,), color: '#30a', },
  { tag: [tags.typeName, tags.namespace,], color: '#085', },
  { tag: tags.className, color: '#167', },
  { tag: [/* @__PURE__ */ tags.special(tags.variableName,), tags.macroName,], color: '#256', },
  { tag: /* @__PURE__ */ tags.definition(tags.propertyName,), color: '#00c', },
  { tag: tags.comment, color: '#940', },
  { tag: tags.invalid, color: '#f00', },
],);
var baseTheme2 = /* @__PURE__ */ EditorView.baseTheme({
  '&.cm-focused .cm-matchingBracket': { backgroundColor: '#328c8252', },
  '&.cm-focused .cm-nonmatchingBracket': { backgroundColor: '#bb555544', },
},);
var DefaultScanDist = 1e4;
var DefaultBrackets = '()[]{}';
var bracketMatchingConfig = /* @__PURE__ */ Facet.define({
  combine(configs,) {
    return combineConfig(configs, {
      afterCursor: true,
      brackets: DefaultBrackets,
      maxScanDistance: DefaultScanDist,
      renderMatch: defaultRenderMatch,
    },);
  },
},);
var matchingMark = /* @__PURE__ */ Decoration.mark({ class: 'cm-matchingBracket', },);
var nonmatchingMark = /* @__PURE__ */ Decoration.mark({ class: 'cm-nonmatchingBracket', },);
function defaultRenderMatch(match,) {
  let decorations2 = [];
  let mark = match.matched ? matchingMark : nonmatchingMark;
  decorations2.push(mark.range(match.start.from, match.start.to,),);
  if (match.end) decorations2.push(mark.range(match.end.from, match.end.to,),);
  return decorations2;
}
var bracketMatchingState = /* @__PURE__ */ StateField.define({
  create() {
    return Decoration.none;
  },
  update(deco, tr,) {
    if (!tr.docChanged && !tr.selection) return deco;
    let decorations2 = [];
    let config = tr.state.facet(bracketMatchingConfig,);
    for (let range of tr.state.selection.ranges) {
      if (!range.empty) continue;
      let match = matchBrackets(tr.state, range.head, -1, config,) ||
        range.head > 0 && matchBrackets(tr.state, range.head - 1, 1, config,) ||
        config.afterCursor &&
          (matchBrackets(tr.state, range.head, 1, config,) ||
            range.head < tr.state.doc.length && matchBrackets(tr.state, range.head + 1, -1, config,));
      if (match) decorations2 = decorations2.concat(config.renderMatch(match, tr.state,),);
    }
    return Decoration.set(decorations2, true,);
  },
  provide: (f,) => EditorView.decorations.from(f,),
},);
var bracketMatchingUnique = [bracketMatchingState, baseTheme2,];
function bracketMatching(config = {},) {
  return [bracketMatchingConfig.of(config,), bracketMatchingUnique,];
}
var bracketMatchingHandle = /* @__PURE__ */ new NodeProp();
function matchingNodes(node, dir, brackets,) {
  let byProp = node.prop(dir < 0 ? NodeProp.openedBy : NodeProp.closedBy,);
  if (byProp) return byProp;
  if (node.name.length == 1) {
    let index = brackets.indexOf(node.name,);
    if (index > -1 && index % 2 == (dir < 0 ? 1 : 0)) return [brackets[index + dir],];
  }
  return null;
}
function findHandle(node,) {
  let hasHandle = node.type.prop(bracketMatchingHandle,);
  return hasHandle ? hasHandle(node.node,) : node;
}
function matchBrackets(state, pos, dir, config = {},) {
  let maxScanDistance = config.maxScanDistance || DefaultScanDist, brackets = config.brackets || DefaultBrackets;
  let tree = syntaxTree(state,), node = tree.resolveInner(pos, dir,);
  for (let cur = node; cur; cur = cur.parent) {
    let matches = matchingNodes(cur.type, dir, brackets,);
    if (matches && cur.from < cur.to) {
      let handle = findHandle(cur,);
      if (handle && (dir > 0 ? pos >= handle.from && pos < handle.to : pos > handle.from && pos <= handle.to)) {
        return matchMarkedBrackets(state, pos, dir, cur, handle, matches, brackets,);
      }
    }
  }
  return matchPlainBrackets(state, pos, dir, tree, node.type, maxScanDistance, brackets,);
}
function matchMarkedBrackets(_state, _pos, dir, token, handle, matching, brackets,) {
  let parent = token.parent, firstToken = { from: handle.from, to: handle.to, };
  let depth = 0, cursor = parent === null || parent === void 0 ? void 0 : parent.cursor();
  if (cursor && (dir < 0 ? cursor.childBefore(token.from,) : cursor.childAfter(token.to,))) {
    do {
      if (dir < 0 ? cursor.to <= token.from : cursor.from >= token.to) {
        if (depth == 0 && matching.indexOf(cursor.type.name,) > -1 && cursor.from < cursor.to) {
          let endHandle = findHandle(cursor,);
          return { start: firstToken, end: endHandle ? { from: endHandle.from, to: endHandle.to, } : void 0, matched: true, };
        } else if (matchingNodes(cursor.type, dir, brackets,)) {
          depth++;
        } else if (matchingNodes(cursor.type, -dir, brackets,)) {
          if (depth == 0) {
            let endHandle1 = findHandle(cursor,);
            return {
              start: firstToken,
              end: endHandle1 && endHandle1.from < endHandle1.to ? { from: endHandle1.from, to: endHandle1.to, } : void 0,
              matched: false,
            };
          }
          depth--;
        }
      }
    } while (dir < 0 ? cursor.prevSibling() : cursor.nextSibling());
  }
  return { start: firstToken, matched: false, };
}
function matchPlainBrackets(state, pos, dir, tree, tokenType, maxScanDistance, brackets,) {
  let startCh = dir < 0 ? state.sliceDoc(pos - 1, pos,) : state.sliceDoc(pos, pos + 1,);
  let bracket2 = brackets.indexOf(startCh,);
  if (bracket2 < 0 || bracket2 % 2 == 0 != dir > 0) return null;
  let startToken = { from: dir < 0 ? pos - 1 : pos, to: dir > 0 ? pos + 1 : pos, };
  let iter = state.doc.iterRange(pos, dir > 0 ? state.doc.length : 0,), depth = 0;
  for (let distance = 0; !iter.next().done && distance <= maxScanDistance;) {
    let text = iter.value;
    if (dir < 0) distance += text.length;
    let basePos = pos + distance * dir;
    for (let pos2 = dir > 0 ? 0 : text.length - 1, end = dir > 0 ? text.length : -1; pos2 != end; pos2 += dir) {
      let found = brackets.indexOf(text[pos2],);
      if (found < 0 || tree.resolveInner(basePos + pos2, 1,).type != tokenType) continue;
      if (found % 2 == 0 == dir > 0) {
        depth++;
      } else if (depth == 1) {
        return { start: startToken, end: { from: basePos + pos2, to: basePos + pos2 + 1, }, matched: found >> 1 == bracket2 >> 1, };
      } else {
        depth--;
      }
    }
    if (dir > 0) distance += text.length;
  }
  return iter.done ? { start: startToken, matched: false, } : null;
}
function countCol(string2, end, tabSize, startIndex = 0, startValue = 0,) {
  if (end == null) {
    end = string2.search(/[^\s\u00a0]/,);
    if (end == -1) end = string2.length;
  }
  let n = startValue;
  for (let i2 = startIndex; i2 < end; i2++) {
    if (string2.charCodeAt(i2,) == 9) n += tabSize - n % tabSize;
    else n++;
  }
  return n;
}
var StringStream = class {
  /**
  True if we are at the end of the line.
  */
  eol() {
    return this.pos >= this.string.length;
  }
  /**
  True if we are at the start of the line.
  */
  sol() {
    return this.pos == 0;
  }
  /**
  Get the next code unit after the current position, or undefined
  if we're at the end of the line.
  */
  peek() {
    return this.string.charAt(this.pos,) || void 0;
  }
  /**
  Read the next code unit and advance `this.pos`.
  */
  next() {
    if (this.pos < this.string.length) return this.string.charAt(this.pos++,);
  }
  /**
  Match the next character against the given string, regular
  expression, or predicate. Consume and return it if it matches.
  */
  eat(match,) {
    let ch = this.string.charAt(this.pos,);
    let ok;
    if (typeof match == 'string') ok = ch == match;
    else ok = ch && (match instanceof RegExp ? match.test(ch,) : match(ch,));
    if (ok) {
      ++this.pos;
      return ch;
    }
  }
  /**
  Continue matching characters that match the given string,
  regular expression, or predicate function. Return true if any
  characters were consumed.
  */
  eatWhile(match,) {
    let start = this.pos;
    while (this.eat(match,)) {
    }
    return this.pos > start;
  }
  /**
  Consume whitespace ahead of `this.pos`. Return true if any was
  found.
  */
  eatSpace() {
    let start = this.pos;
    while (/[\s\u00a0]/.test(this.string.charAt(this.pos,),)) ++this.pos;
    return this.pos > start;
  }
  /**
  Move to the end of the line.
  */
  skipToEnd() {
    this.pos = this.string.length;
  }
  /**
  Move to directly before the given character, if found on the
  current line.
  */
  skipTo(ch,) {
    let found = this.string.indexOf(ch, this.pos,);
    if (found > -1) {
      this.pos = found;
      return true;
    }
  }
  /**
  Move back `n` characters.
  */
  backUp(n,) {
    this.pos -= n;
  }
  /**
  Get the column position at `this.pos`.
  */
  column() {
    if (this.lastColumnPos < this.start) {
      this.lastColumnValue = countCol(this.string, this.start, this.tabSize, this.lastColumnPos, this.lastColumnValue,);
      this.lastColumnPos = this.start;
    }
    return this.lastColumnValue;
  }
  /**
  Get the indentation column of the current line.
  */
  indentation() {
    var _a2;
    return (_a2 = this.overrideIndent) !== null && _a2 !== void 0 ? _a2 : countCol(this.string, null, this.tabSize,);
  }
  /**
  Match the input against the given string or regular expression
  (which should start with a `^`). Return true or the regexp match
  if it matches.

  Unless `consume` is set to `false`, this will move `this.pos`
  past the matched text.

  When matching a string `caseInsensitive` can be set to true to
  make the match case-insensitive.
  */
  match(pattern, consume, caseInsensitive,) {
    if (typeof pattern == 'string') {
      let cased = (str,) => caseInsensitive ? str.toLowerCase() : str;
      let substr = this.string.substr(this.pos, pattern.length,);
      if (cased(substr,) == cased(pattern,)) {
        if (consume !== false) this.pos += pattern.length;
        return true;
      } else return null;
    } else {
      let match = this.string.slice(this.pos,).match(pattern,);
      if (match && match.index > 0) return null;
      if (match && consume !== false) this.pos += match[0].length;
      return match;
    }
  }
  /**
  Get the current token.
  */
  current() {
    return this.string.slice(this.start, this.pos,);
  }
  /**
  Create a stream.
  */
  constructor(string2, tabSize, indentUnit2, overrideIndent,) {
    this.string = string2;
    this.tabSize = tabSize;
    this.indentUnit = indentUnit2;
    this.overrideIndent = overrideIndent;
    this.pos = 0;
    this.start = 0;
    this.lastColumnPos = 0;
    this.lastColumnValue = 0;
  }
};
function fullParser(spec,) {
  return {
    name: spec.name || '',
    token: spec.token,
    blankLine: spec.blankLine || (() => {
    }),
    startState: spec.startState || (() => true),
    copyState: spec.copyState || defaultCopyState,
    indent: spec.indent || (() => null),
    languageData: spec.languageData || {},
    tokenTable: spec.tokenTable || noTokens,
  };
}
function defaultCopyState(state,) {
  if (typeof state != 'object') return state;
  let newState = {};
  for (let prop in state) {
    let val = state[prop];
    newState[prop] = val instanceof Array ? val.slice() : val;
  }
  return newState;
}
var IndentedFrom = /* @__PURE__ */ new WeakMap();
var StreamLanguage = class extends Language {
  /**
  Define a stream language.
  */
  static define(spec,) {
    return new StreamLanguage(spec,);
  }
  getIndent(cx, pos,) {
    let tree = syntaxTree(cx.state,), at = tree.resolve(pos,);
    while (at && at.type != this.topNode) at = at.parent;
    if (!at) return null;
    let from = void 0;
    let { overrideIndentation, } = cx.options;
    if (overrideIndentation) {
      from = IndentedFrom.get(cx.state,);
      if (from != null && from < pos - 1e4) from = void 0;
    }
    let start = findState(this, tree, 0, at.from, from !== null && from !== void 0 ? from : pos,), statePos, state;
    if (start) {
      state = start.state;
      statePos = start.pos + 1;
    } else {
      state = this.streamParser.startState(cx.unit,);
      statePos = 0;
    }
    if (pos - statePos > 1e4) return null;
    while (statePos < pos) {
      let line2 = cx.state.doc.lineAt(statePos,), end = Math.min(pos, line2.to,);
      if (line2.length) {
        let indentation = overrideIndentation ? overrideIndentation(line2.from,) : -1;
        let stream = new StringStream(line2.text, cx.state.tabSize, cx.unit, indentation < 0 ? void 0 : indentation,);
        while (stream.pos < end - line2.from) readToken(this.streamParser.token, stream, state,);
      } else {
        this.streamParser.blankLine(state, cx.unit,);
      }
      if (end == pos) break;
      statePos = line2.to + 1;
    }
    let line = cx.lineAt(pos,);
    if (overrideIndentation && from == null) IndentedFrom.set(cx.state, line.from,);
    return this.streamParser.indent(state, /^\s*(.*)/.exec(line.text,)[1], cx,);
  }
  get allowsNesting() {
    return false;
  }
  constructor(parser,) {
    let data = defineLanguageFacet(parser.languageData,);
    let p = fullParser(parser,), self;
    let impl = new class extends Parser {
      createParse(input, fragments, ranges,) {
        return new Parse(self, input, fragments, ranges,);
      }
    }();
    super(data, impl, [indentService.of((cx, pos,) => this.getIndent(cx, pos,)),], parser.name,);
    this.topNode = docID(data,);
    self = this;
    this.streamParser = p;
    this.stateAfter = new NodeProp({ perNode: true, },);
    this.tokenTable = parser.tokenTable ? new TokenTable(p.tokenTable,) : defaultTokenTable;
  }
};
function findState(lang, tree, off, startPos, before,) {
  let state = off >= startPos && off + tree.length <= before && tree.prop(lang.stateAfter,);
  if (state) return { state: lang.streamParser.copyState(state,), pos: off + tree.length, };
  for (let i2 = tree.children.length - 1; i2 >= 0; i2--) {
    let child = tree.children[i2], pos = off + tree.positions[i2];
    let found = child instanceof Tree && pos < before && findState(lang, child, pos, startPos, before,);
    if (found) return found;
  }
  return null;
}
function cutTree(lang, tree, from, to, inside2,) {
  if (inside2 && from <= 0 && to >= tree.length) return tree;
  if (!inside2 && tree.type == lang.topNode) inside2 = true;
  for (let i2 = tree.children.length - 1; i2 >= 0; i2--) {
    let pos = tree.positions[i2], child = tree.children[i2], inner;
    if (pos < to && child instanceof Tree) {
      if (!(inner = cutTree(lang, child, from - pos, to - pos, inside2,))) break;
      return !inside2
        ? inner
        : new Tree(tree.type, tree.children.slice(0, i2,).concat(inner,), tree.positions.slice(0, i2 + 1,), pos + inner.length,);
    }
  }
  return null;
}
function findStartInFragments(lang, fragments, startPos, editorState,) {
  for (let f of fragments) {
    let from = f.from + (f.openStart ? 25 : 0), to = f.to - (f.openEnd ? 25 : 0);
    let found = from <= startPos && to > startPos && findState(lang, f.tree, 0 - f.offset, startPos, to,), tree;
    if (found && (tree = cutTree(lang, f.tree, startPos + f.offset, found.pos + f.offset, false,))) return { state: found.state, tree, };
  }
  return { state: lang.streamParser.startState(editorState ? getIndentUnit(editorState,) : 4,), tree: Tree.empty, };
}
var Parse = class {
  advance() {
    let context = ParseContext.get();
    let parseEnd = this.stoppedAt == null ? this.to : Math.min(this.to, this.stoppedAt,);
    let end = Math.min(parseEnd, this.chunkStart + 2048,);
    if (context) end = Math.min(end, context.viewport.to,);
    while (this.parsedPos < end) this.parseLine(context,);
    if (this.chunkStart < this.parsedPos) this.finishChunk();
    if (this.parsedPos >= parseEnd) return this.finish();
    if (context && this.parsedPos >= context.viewport.to) {
      context.skipUntilInView(this.parsedPos, parseEnd,);
      return this.finish();
    }
    return null;
  }
  stopAt(pos,) {
    this.stoppedAt = pos;
  }
  lineAfter(pos,) {
    let chunk = this.input.chunk(pos,);
    if (!this.input.lineChunks) {
      let eol = chunk.indexOf('\n',);
      if (eol > -1) chunk = chunk.slice(0, eol,);
    } else if (chunk == '\n') {
      chunk = '';
    }
    return pos + chunk.length <= this.to ? chunk : chunk.slice(0, this.to - pos,);
  }
  nextLine() {
    let from = this.parsedPos, line = this.lineAfter(from,), end = from + line.length;
    for (let index = this.rangeIndex;;) {
      let rangeEnd = this.ranges[index].to;
      if (rangeEnd >= end) break;
      line = line.slice(0, rangeEnd - (end - line.length),);
      index++;
      if (index == this.ranges.length) break;
      let rangeStart = this.ranges[index].from;
      let after = this.lineAfter(rangeStart,);
      line += after;
      end = rangeStart + after.length;
    }
    return { line, end, };
  }
  skipGapsTo(pos, offset, side,) {
    for (;;) {
      let end = this.ranges[this.rangeIndex].to, offPos = pos + offset;
      if (side > 0 ? end > offPos : end >= offPos) break;
      let start = this.ranges[++this.rangeIndex].from;
      offset += start - end;
    }
    return offset;
  }
  moveRangeIndex() {
    while (this.ranges[this.rangeIndex].to < this.parsedPos) this.rangeIndex++;
  }
  emitToken(id2, from, to, size, offset,) {
    if (this.ranges.length > 1) {
      offset = this.skipGapsTo(from, offset, 1,);
      from += offset;
      let len0 = this.chunk.length;
      offset = this.skipGapsTo(to, offset, -1,);
      to += offset;
      size += this.chunk.length - len0;
    }
    this.chunk.push(id2, from, to, size,);
    return offset;
  }
  parseLine(context,) {
    let { line, end, } = this.nextLine(), offset = 0, { streamParser, } = this.lang;
    let stream = new StringStream(line, context ? context.state.tabSize : 4, context ? getIndentUnit(context.state,) : 2,);
    if (stream.eol()) {
      streamParser.blankLine(this.state, stream.indentUnit,);
    } else {
      while (!stream.eol()) {
        let token = readToken(streamParser.token, stream, this.state,);
        if (token) {
          offset = this.emitToken(
            this.lang.tokenTable.resolve(token,),
            this.parsedPos + stream.start,
            this.parsedPos + stream.pos,
            4,
            offset,
          );
        }
        if (stream.start > 1e4) break;
      }
    }
    this.parsedPos = end;
    this.moveRangeIndex();
    if (this.parsedPos < this.to) this.parsedPos++;
  }
  finishChunk() {
    let tree = Tree.build({
      buffer: this.chunk,
      start: this.chunkStart,
      length: this.parsedPos - this.chunkStart,
      nodeSet,
      topID: 0,
      maxBufferLength: 2048,
      reused: this.chunkReused,
    },);
    tree = new Tree(tree.type, tree.children, tree.positions, tree.length, [[
      this.lang.stateAfter,
      this.lang.streamParser.copyState(this.state,),
    ],],);
    this.chunks.push(tree,);
    this.chunkPos.push(this.chunkStart - this.ranges[0].from,);
    this.chunk = [];
    this.chunkReused = void 0;
    this.chunkStart = this.parsedPos;
  }
  finish() {
    return new Tree(this.lang.topNode, this.chunks, this.chunkPos, this.parsedPos - this.ranges[0].from,).balance();
  }
  constructor(lang, input, fragments, ranges,) {
    this.lang = lang;
    this.input = input;
    this.fragments = fragments;
    this.ranges = ranges;
    this.stoppedAt = null;
    this.chunks = [];
    this.chunkPos = [];
    this.chunk = [];
    this.chunkReused = void 0;
    this.rangeIndex = 0;
    this.to = ranges[ranges.length - 1].to;
    let context = ParseContext.get(), from = ranges[0].from;
    let { state, tree, } = findStartInFragments(lang, fragments, from, context === null || context === void 0 ? void 0 : context.state,);
    this.state = state;
    this.parsedPos = this.chunkStart = from + tree.length;
    for (let i2 = 0; i2 < tree.children.length; i2++) {
      this.chunks.push(tree.children[i2],);
      this.chunkPos.push(tree.positions[i2],);
    }
    if (context && this.parsedPos < context.viewport.from - 1e5) {
      this.state = this.lang.streamParser.startState(getIndentUnit(context.state,),);
      context.skipUntilInView(this.parsedPos, context.viewport.from,);
      this.parsedPos = context.viewport.from;
    }
    this.moveRangeIndex();
  }
};
function readToken(token, stream, state,) {
  stream.start = stream.pos;
  for (let i2 = 0; i2 < 10; i2++) {
    let result = token(stream, state,);
    if (stream.pos > stream.start) return result;
  }
  throw new Error('Stream parser failed to advance stream.',);
}
var noTokens = /* @__PURE__ */ Object.create(null,);
var typeArray = [NodeType.none,];
var nodeSet = /* @__PURE__ */ new NodeSet(typeArray,);
var warned = [];
var defaultTable = /* @__PURE__ */ Object.create(null,);
for (
  let [legacyName, name2,] of [
    ['variable', 'variableName',],
    ['variable-2', 'variableName.special',],
    ['string-2', 'string.special',],
    ['def', 'variableName.definition',],
    ['tag', 'tagName',],
    ['attribute', 'attributeName',],
    ['type', 'typeName',],
    ['builtin', 'variableName.standard',],
    ['qualifier', 'modifier',],
    ['error', 'invalid',],
    ['header', 'heading',],
    ['property', 'propertyName',],
  ]
) defaultTable[legacyName] = /* @__PURE__ */ createTokenType(noTokens, name2,);
var TokenTable = class {
  resolve(tag,) {
    return !tag ? 0 : this.table[tag] || (this.table[tag] = createTokenType(this.extra, tag,));
  }
  constructor(extra,) {
    this.extra = extra;
    this.table = Object.assign(/* @__PURE__ */ Object.create(null,), defaultTable,);
  }
};
var defaultTokenTable = /* @__PURE__ */ new TokenTable(noTokens,);
function warnForPart(part, msg,) {
  if (warned.indexOf(part,) > -1) return;
  warned.push(part,);
  console.warn(msg,);
}
function createTokenType(extra, tagStr,) {
  let tag = null;
  for (let part of tagStr.split('.',)) {
    let value = extra[part] || tags[part];
    if (!value) {
      warnForPart(part, `Unknown highlighting tag ${part}`,);
    } else if (typeof value == 'function') {
      if (!tag) warnForPart(part, `Modifier ${part} used at start of tag`,);
      else tag = value(tag,);
    } else {
      if (tag) warnForPart(part, `Tag ${part} used as modifier`,);
      else tag = value;
    }
  }
  if (!tag) return 0;
  let name2 = tagStr.replace(/ /g, '_',),
    type = NodeType.define({ id: typeArray.length, name: name2, props: [styleTags({ [name2]: tag, },),], },);
  typeArray.push(type,);
  return type.id;
}
function docID(data,) {
  let type = NodeType.define({ id: typeArray.length, name: 'Document', props: [languageDataProp.add(() => data),], },);
  typeArray.push(type,);
  return type;
}

// https :https://framerusercontent.com/modules/MiFWyNBz6FaRwOOEYJ4H/RMT2Q4ScDAUq3xYhahWs/lezer_lr.js
var Stack = class {
  /// @internal
  toString() {
    return `[${this.stack.filter((_, i2,) => i2 % 3 == 0).concat(this.state,)}]@${this.pos}${this.score ? '!' + this.score : ''}`;
  }
  // Start an empty stack
  /// @internal
  static start(p, state, pos = 0,) {
    let cx = p.parser.context;
    return new Stack(p, [], state, pos, pos, 0, [], 0, cx ? new StackContext(cx, cx.start,) : null, 0, null,);
  }
  /// The stack's current [context](#lr.ContextTracker) value, if
  /// any. Its type will depend on the context tracker's type
  /// parameter, or it will be `null` if there is no context
  /// tracker.
  get context() {
    return this.curContext ? this.curContext.context : null;
  }
  // Push a state onto the stack, tracking its start position as well
  // as the buffer base at that point.
  /// @internal
  pushState(state, start,) {
    this.stack.push(this.state, start, this.bufferBase + this.buffer.length,);
    this.state = state;
  }
  // Apply a reduce action
  /// @internal
  reduce(action,) {
    var _a2;
    let depth = action >> 19, type = action & 65535;
    let { parser, } = this.p;
    let dPrec = parser.dynamicPrecedence(type,);
    if (dPrec) this.score += dPrec;
    if (depth == 0) {
      this.pushState(parser.getGoto(this.state, type, true,), this.reducePos,);
      if (type < parser.minRepeatTerm) this.storeNode(type, this.reducePos, this.reducePos, 4, true,);
      this.reduceContext(type, this.reducePos,);
      return;
    }
    let base2 = this.stack.length - (depth - 1) * 3 - (action & 262144 ? 6 : 0);
    let start = base2 ? this.stack[base2 - 2] : this.p.ranges[0].from, size = this.reducePos - start;
    if (size >= 2e3 && !((_a2 = this.p.parser.nodeSet.types[type]) === null || _a2 === void 0 ? void 0 : _a2.isAnonymous)) {
      if (start == this.p.lastBigReductionStart) {
        this.p.bigReductionCount++;
        this.p.lastBigReductionSize = size;
      } else if (this.p.lastBigReductionSize < size) {
        this.p.bigReductionCount = 1;
        this.p.lastBigReductionStart = start;
        this.p.lastBigReductionSize = size;
      }
    }
    let bufferBase = base2 ? this.stack[base2 - 1] : 0, count = this.bufferBase + this.buffer.length - bufferBase;
    if (type < parser.minRepeatTerm || action & 131072) {
      let pos = parser.stateFlag(this.state, 1,) ? this.pos : this.reducePos;
      this.storeNode(type, start, pos, count + 4, true,);
    }
    if (action & 262144) {
      this.state = this.stack[base2];
    } else {
      let baseStateID = this.stack[base2 - 3];
      this.state = parser.getGoto(baseStateID, type, true,);
    }
    while (this.stack.length > base2) this.stack.pop();
    this.reduceContext(type, start,);
  }
  // Shift a value into the buffer
  /// @internal
  storeNode(term, start, end, size = 4, isReduce = false,) {
    if (term == 0 && (!this.stack.length || this.stack[this.stack.length - 1] < this.buffer.length + this.bufferBase)) {
      let cur = this, top3 = this.buffer.length;
      if (top3 == 0 && cur.parent) {
        top3 = cur.bufferBase - cur.parent.bufferBase;
        cur = cur.parent;
      }
      if (top3 > 0 && cur.buffer[top3 - 4] == 0 && cur.buffer[top3 - 1] > -1) {
        if (start == end) return;
        if (cur.buffer[top3 - 2] >= start) {
          cur.buffer[top3 - 2] = end;
          return;
        }
      }
    }
    if (!isReduce || this.pos == end) {
      this.buffer.push(term, start, end, size,);
    } else {
      let index = this.buffer.length;
      if (index > 0 && this.buffer[index - 4] != 0) {
        while (index > 0 && this.buffer[index - 2] > end) {
          this.buffer[index] = this.buffer[index - 4];
          this.buffer[index + 1] = this.buffer[index - 3];
          this.buffer[index + 2] = this.buffer[index - 2];
          this.buffer[index + 3] = this.buffer[index - 1];
          index -= 4;
          if (size > 4) size -= 4;
        }
      }
      this.buffer[index] = term;
      this.buffer[index + 1] = start;
      this.buffer[index + 2] = end;
      this.buffer[index + 3] = size;
    }
  }
  // Apply a shift action
  /// @internal
  shift(action, next, nextEnd,) {
    let start = this.pos;
    if (action & 131072) {
      this.pushState(action & 65535, this.pos,);
    } else if ((action & 262144) == 0) {
      let nextState = action, { parser, } = this.p;
      if (nextEnd > this.pos || next <= parser.maxNode) {
        this.pos = nextEnd;
        if (!parser.stateFlag(nextState, 1,)) this.reducePos = nextEnd;
      }
      this.pushState(nextState, start,);
      this.shiftContext(next, start,);
      if (next <= parser.maxNode) this.buffer.push(next, start, nextEnd, 4,);
    } else {
      this.pos = nextEnd;
      this.shiftContext(next, start,);
      if (next <= this.p.parser.maxNode) this.buffer.push(next, start, nextEnd, 4,);
    }
  }
  // Apply an action
  /// @internal
  apply(action, next, nextEnd,) {
    if (action & 65536) this.reduce(action,);
    else this.shift(action, next, nextEnd,);
  }
  // Add a prebuilt (reused) node into the buffer.
  /// @internal
  useNode(value, next,) {
    let index = this.p.reused.length - 1;
    if (index < 0 || this.p.reused[index] != value) {
      this.p.reused.push(value,);
      index++;
    }
    let start = this.pos;
    this.reducePos = this.pos = start + value.length;
    this.pushState(next, start,);
    this.buffer.push(index, start, this.reducePos, -1,);
    if (this.curContext) {
      this.updateContext(
        this.curContext.tracker.reuse(this.curContext.context, value, this, this.p.stream.reset(this.pos - value.length,),),
      );
    }
  }
  // Split the stack. Due to the buffer sharing and the fact
  // that `this.stack` tends to stay quite shallow, this isn't very
  // expensive.
  /// @internal
  split() {
    let parent = this;
    let off = parent.buffer.length;
    while (off > 0 && parent.buffer[off - 2] > parent.reducePos) off -= 4;
    let buffer = parent.buffer.slice(off,), base2 = parent.bufferBase + off;
    while (parent && base2 == parent.bufferBase) parent = parent.parent;
    return new Stack(
      this.p,
      this.stack.slice(),
      this.state,
      this.reducePos,
      this.pos,
      this.score,
      buffer,
      base2,
      this.curContext,
      this.lookAhead,
      parent,
    );
  }
  // Try to recover from an error by 'deleting' (ignoring) one token.
  /// @internal
  recoverByDelete(next, nextEnd,) {
    let isNode = next <= this.p.parser.maxNode;
    if (isNode) this.storeNode(next, this.pos, nextEnd, 4,);
    this.storeNode(0, this.pos, nextEnd, isNode ? 8 : 4,);
    this.pos = this.reducePos = nextEnd;
    this.score -= 190;
  }
  /// Check if the given term would be able to be shifted (optionally
  /// after some reductions) on this stack. This can be useful for
  /// external tokenizers that want to make sure they only provide a
  /// given token when it applies.
  canShift(term,) {
    for (let sim = new SimulatedStack(this,);;) {
      let action = this.p.parser.stateSlot(sim.state, 4,) || this.p.parser.hasAction(sim.state, term,);
      if (action == 0) return false;
      if ((action & 65536) == 0) return true;
      sim.reduce(action,);
    }
  }
  // Apply up to Recover.MaxNext recovery actions that conceptually
  // inserts some missing token or rule.
  /// @internal
  recoverByInsert(next,) {
    if (this.stack.length >= 300) return [];
    let nextStates = this.p.parser.nextStates(this.state,);
    if (nextStates.length > 4 << 1 || this.stack.length >= 120) {
      let best = [];
      for (let i2 = 0, s; i2 < nextStates.length; i2 += 2) {
        if ((s = nextStates[i2 + 1]) != this.state && this.p.parser.hasAction(s, next,)) best.push(nextStates[i2], s,);
      }
      if (this.stack.length < 120) {
        for (let i1 = 0; best.length < 4 << 1 && i1 < nextStates.length; i1 += 2) {
          let s1 = nextStates[i1 + 1];
          if (!best.some((v, i2,) => i2 & 1 && v == s1)) best.push(nextStates[i1], s1,);
        }
      }
      nextStates = best;
    }
    let result = [];
    for (let i2 = 0; i2 < nextStates.length && result.length < 4; i2 += 2) {
      let s2 = nextStates[i2 + 1];
      if (s2 == this.state) continue;
      let stack = this.split();
      stack.pushState(s2, this.pos,);
      stack.storeNode(0, stack.pos, stack.pos, 4, true,);
      stack.shiftContext(nextStates[i2], this.pos,);
      stack.score -= 200;
      result.push(stack,);
    }
    return result;
  }
  // Force a reduce, if possible. Return false if that can't
  // be done.
  /// @internal
  forceReduce() {
    let { parser, } = this.p;
    let reduce = parser.stateSlot(this.state, 5,);
    if ((reduce & 65536) == 0) return false;
    if (!parser.validAction(this.state, reduce,)) {
      let depth = reduce >> 19, term = reduce & 65535;
      let target = this.stack.length - depth * 3;
      if (target < 0 || parser.getGoto(this.stack[target], term, false,) < 0) {
        let backup = this.findForcedReduction();
        if (backup == null) return false;
        reduce = backup;
      }
      this.storeNode(0, this.reducePos, this.reducePos, 4, true,);
      this.score -= 100;
    }
    this.reducePos = this.pos;
    this.reduce(reduce,);
    return true;
  }
  /// Try to scan through the automaton to find some kind of reduction
  /// that can be applied. Used when the regular ForcedReduce field
  /// isn't a valid action. @internal
  findForcedReduction() {
    let { parser, } = this.p, seen = [];
    let explore = (state, depth,) => {
      if (seen.includes(state,)) return;
      seen.push(state,);
      return parser.allActions(state, (action,) => {
        if (action & (262144 | 131072));
        else if (action & 65536) {
          let rDepth = (action >> 19) - depth;
          if (rDepth > 1) {
            let term = action & 65535, target = this.stack.length - rDepth * 3;
            if (target >= 0 && parser.getGoto(this.stack[target], term, false,) >= 0) return rDepth << 19 | 65536 | term;
          }
        } else {
          let found = explore(action, depth + 1,);
          if (found != null) return found;
        }
      },);
    };
    return explore(this.state, 0,);
  }
  /// @internal
  forceAll() {
    while (!this.p.parser.stateFlag(this.state, 2,)) {
      if (!this.forceReduce()) {
        this.storeNode(0, this.pos, this.pos, 4, true,);
        break;
      }
    }
    return this;
  }
  /// Check whether this state has no further actions (assumed to be a direct descendant of the
  /// top state, since any other states must be able to continue
  /// somehow). @internal
  get deadEnd() {
    if (this.stack.length != 3) return false;
    let { parser, } = this.p;
    return parser.data[parser.stateSlot(this.state, 1,)] == 65535 && !parser.stateSlot(this.state, 4,);
  }
  /// Restart the stack (put it back in its start state). Only safe
  /// when this.stack.length == 3 (state is directly below the top
  /// state). @internal
  restart() {
    this.state = this.stack[0];
    this.stack.length = 0;
  }
  /// @internal
  sameState(other,) {
    if (this.state != other.state || this.stack.length != other.stack.length) return false;
    for (let i2 = 0; i2 < this.stack.length; i2 += 3) if (this.stack[i2] != other.stack[i2]) return false;
    return true;
  }
  /// Get the parser used by this stack.
  get parser() {
    return this.p.parser;
  }
  /// Test whether a given dialect (by numeric ID, as exported from
  /// the terms file) is enabled.
  dialectEnabled(dialectID,) {
    return this.p.parser.dialect.flags[dialectID];
  }
  shiftContext(term, start,) {
    if (this.curContext) {
      this.updateContext(this.curContext.tracker.shift(this.curContext.context, term, this, this.p.stream.reset(start,),),);
    }
  }
  reduceContext(term, start,) {
    if (this.curContext) {
      this.updateContext(this.curContext.tracker.reduce(this.curContext.context, term, this, this.p.stream.reset(start,),),);
    }
  }
  /// @internal
  emitContext() {
    let last = this.buffer.length - 1;
    if (last < 0 || this.buffer[last] != -3) this.buffer.push(this.curContext.hash, this.pos, this.pos, -3,);
  }
  /// @internal
  emitLookAhead() {
    let last = this.buffer.length - 1;
    if (last < 0 || this.buffer[last] != -4) this.buffer.push(this.lookAhead, this.pos, this.pos, -4,);
  }
  updateContext(context,) {
    if (context != this.curContext.context) {
      let newCx = new StackContext(this.curContext.tracker, context,);
      if (newCx.hash != this.curContext.hash) this.emitContext();
      this.curContext = newCx;
    }
  }
  /// @internal
  setLookAhead(lookAhead,) {
    if (lookAhead > this.lookAhead) {
      this.emitLookAhead();
      this.lookAhead = lookAhead;
    }
  }
  /// @internal
  close() {
    if (this.curContext && this.curContext.tracker.strict) this.emitContext();
    if (this.lookAhead > 0) this.emitLookAhead();
  }
  /// @internal
  constructor(p, stack, state, reducePos, pos, score, buffer, bufferBase, curContext, lookAhead = 0, parent,) {
    this.p = p;
    this.stack = stack;
    this.state = state;
    this.reducePos = reducePos;
    this.pos = pos;
    this.score = score;
    this.buffer = buffer;
    this.bufferBase = bufferBase;
    this.curContext = curContext;
    this.lookAhead = lookAhead;
    this.parent = parent;
  }
};
var StackContext = class {
  constructor(tracker, context,) {
    this.tracker = tracker;
    this.context = context;
    this.hash = tracker.strict ? tracker.hash(context,) : 0;
  }
};
var Recover;
(function (Recover2,) {
  Recover2[Recover2['Insert'] = 200] = 'Insert';
  Recover2[Recover2['Delete'] = 190] = 'Delete';
  Recover2[Recover2['Reduce'] = 100] = 'Reduce';
  Recover2[Recover2['MaxNext'] = 4] = 'MaxNext';
  Recover2[Recover2['MaxInsertStackDepth'] = 300] = 'MaxInsertStackDepth';
  Recover2[Recover2['DampenInsertStackDepth'] = 120] = 'DampenInsertStackDepth';
  Recover2[Recover2['MinBigReduction'] = 2e3] = 'MinBigReduction';
})(Recover || (Recover = {}),);
var SimulatedStack = class {
  reduce(action,) {
    let term = action & 65535, depth = action >> 19;
    if (depth == 0) {
      if (this.stack == this.start.stack) this.stack = this.stack.slice();
      this.stack.push(this.state, 0, 0,);
      this.base += 3;
    } else {
      this.base -= (depth - 1) * 3;
    }
    let goto = this.start.p.parser.getGoto(this.stack[this.base - 3], term, true,);
    this.state = goto;
  }
  constructor(start,) {
    this.start = start;
    this.state = start.state;
    this.stack = start.stack;
    this.base = this.stack.length;
  }
};
var StackBufferCursor = class {
  static create(stack, pos = stack.bufferBase + stack.buffer.length,) {
    return new StackBufferCursor(stack, pos, pos - stack.bufferBase,);
  }
  maybeNext() {
    let next = this.stack.parent;
    if (next != null) {
      this.index = this.stack.bufferBase - next.bufferBase;
      this.stack = next;
      this.buffer = next.buffer;
    }
  }
  get id() {
    return this.buffer[this.index - 4];
  }
  get start() {
    return this.buffer[this.index - 3];
  }
  get end() {
    return this.buffer[this.index - 2];
  }
  get size() {
    return this.buffer[this.index - 1];
  }
  next() {
    this.index -= 4;
    this.pos -= 4;
    if (this.index == 0) this.maybeNext();
  }
  fork() {
    return new StackBufferCursor(this.stack, this.pos, this.index,);
  }
  constructor(stack, pos, index,) {
    this.stack = stack;
    this.pos = pos;
    this.index = index;
    this.buffer = stack.buffer;
    if (this.index == 0) this.maybeNext();
  }
};
function decodeArray(input, Type = Uint16Array,) {
  if (typeof input != 'string') return input;
  let array = null;
  for (let pos = 0, out = 0; pos < input.length;) {
    let value = 0;
    for (;;) {
      let next = input.charCodeAt(pos++,), stop = false;
      if (next == 126) {
        value = 65535;
        break;
      }
      if (next >= 92) next--;
      if (next >= 34) next--;
      let digit = next - 32;
      if (digit >= 46) {
        digit -= 46;
        stop = true;
      }
      value += digit;
      if (stop) break;
      value *= 46;
    }
    if (array) array[out++] = value;
    else array = new Type(value,);
  }
  return array;
}
var CachedToken = class {
  constructor() {
    this.start = -1;
    this.value = -1;
    this.end = -1;
    this.extended = -1;
    this.lookAhead = 0;
    this.mask = 0;
    this.context = 0;
  }
};
var nullToken = new CachedToken();
var InputStream = class {
  /// @internal
  resolveOffset(offset, assoc,) {
    let range = this.range, index = this.rangeIndex;
    let pos = this.pos + offset;
    while (pos < range.from) {
      if (!index) return null;
      let next = this.ranges[--index];
      pos -= range.from - next.to;
      range = next;
    }
    while (assoc < 0 ? pos > range.to : pos >= range.to) {
      if (index == this.ranges.length - 1) return null;
      let next1 = this.ranges[++index];
      pos += next1.from - range.to;
      range = next1;
    }
    return pos;
  }
  /// @internal
  clipPos(pos,) {
    if (pos >= this.range.from && pos < this.range.to) return pos;
    for (let range of this.ranges) if (range.to > pos) return Math.max(pos, range.from,);
    return this.end;
  }
  /// Look at a code unit near the stream position. `.peek(0)` equals
  /// `.next`, `.peek(-1)` gives you the previous character, and so
  /// on.
  ///
  /// Note that looking around during tokenizing creates dependencies
  /// on potentially far-away content, which may reduce the
  /// effectiveness incremental parsing—when looking forward—or even
  /// cause invalid reparses when looking backward more than 25 code
  /// units, since the library does not track lookbehind.
  peek(offset,) {
    let idx = this.chunkOff + offset, pos, result;
    if (idx >= 0 && idx < this.chunk.length) {
      pos = this.pos + offset;
      result = this.chunk.charCodeAt(idx,);
    } else {
      let resolved = this.resolveOffset(offset, 1,);
      if (resolved == null) return -1;
      pos = resolved;
      if (pos >= this.chunk2Pos && pos < this.chunk2Pos + this.chunk2.length) {
        result = this.chunk2.charCodeAt(pos - this.chunk2Pos,);
      } else {
        let i2 = this.rangeIndex, range = this.range;
        while (range.to <= pos) range = this.ranges[++i2];
        this.chunk2 = this.input.chunk(this.chunk2Pos = pos,);
        if (pos + this.chunk2.length > range.to) this.chunk2 = this.chunk2.slice(0, range.to - pos,);
        result = this.chunk2.charCodeAt(0,);
      }
    }
    if (pos >= this.token.lookAhead) this.token.lookAhead = pos + 1;
    return result;
  }
  /// Accept a token. By default, the end of the token is set to the
  /// current stream position, but you can pass an offset (relative to
  /// the stream position) to change that.
  acceptToken(token, endOffset = 0,) {
    let end = endOffset ? this.resolveOffset(endOffset, -1,) : this.pos;
    if (end == null || end < this.token.start) throw new RangeError('Token end out of bounds',);
    this.token.value = token;
    this.token.end = end;
  }
  getChunk() {
    if (this.pos >= this.chunk2Pos && this.pos < this.chunk2Pos + this.chunk2.length) {
      let { chunk, chunkPos, } = this;
      this.chunk = this.chunk2;
      this.chunkPos = this.chunk2Pos;
      this.chunk2 = chunk;
      this.chunk2Pos = chunkPos;
      this.chunkOff = this.pos - this.chunkPos;
    } else {
      this.chunk2 = this.chunk;
      this.chunk2Pos = this.chunkPos;
      let nextChunk = this.input.chunk(this.pos,);
      let end = this.pos + nextChunk.length;
      this.chunk = end > this.range.to ? nextChunk.slice(0, this.range.to - this.pos,) : nextChunk;
      this.chunkPos = this.pos;
      this.chunkOff = 0;
    }
  }
  readNext() {
    if (this.chunkOff >= this.chunk.length) {
      this.getChunk();
      if (this.chunkOff == this.chunk.length) return this.next = -1;
    }
    return this.next = this.chunk.charCodeAt(this.chunkOff,);
  }
  /// Move the stream forward N (defaults to 1) code units. Returns
  /// the new value of [`next`](#lr.InputStream.next).
  advance(n = 1,) {
    this.chunkOff += n;
    while (this.pos + n >= this.range.to) {
      if (this.rangeIndex == this.ranges.length - 1) return this.setDone();
      n -= this.range.to - this.pos;
      this.range = this.ranges[++this.rangeIndex];
      this.pos = this.range.from;
    }
    this.pos += n;
    if (this.pos >= this.token.lookAhead) this.token.lookAhead = this.pos + 1;
    return this.readNext();
  }
  setDone() {
    this.pos = this.chunkPos = this.end;
    this.range = this.ranges[this.rangeIndex = this.ranges.length - 1];
    this.chunk = '';
    return this.next = -1;
  }
  /// @internal
  reset(pos, token,) {
    if (token) {
      this.token = token;
      token.start = pos;
      token.lookAhead = pos + 1;
      token.value = token.extended = -1;
    } else {
      this.token = nullToken;
    }
    if (this.pos != pos) {
      this.pos = pos;
      if (pos == this.end) {
        this.setDone();
        return this;
      }
      while (pos < this.range.from) this.range = this.ranges[--this.rangeIndex];
      while (pos >= this.range.to) this.range = this.ranges[++this.rangeIndex];
      if (pos >= this.chunkPos && pos < this.chunkPos + this.chunk.length) {
        this.chunkOff = pos - this.chunkPos;
      } else {
        this.chunk = '';
        this.chunkOff = 0;
      }
      this.readNext();
    }
    return this;
  }
  /// @internal
  read(from, to,) {
    if (from >= this.chunkPos && to <= this.chunkPos + this.chunk.length) {
      return this.chunk.slice(from - this.chunkPos, to - this.chunkPos,);
    }
    if (from >= this.chunk2Pos && to <= this.chunk2Pos + this.chunk2.length) {
      return this.chunk2.slice(from - this.chunk2Pos, to - this.chunk2Pos,);
    }
    if (from >= this.range.from && to <= this.range.to) return this.input.read(from, to,);
    let result = '';
    for (let r of this.ranges) {
      if (r.from >= to) break;
      if (r.to > from) result += this.input.read(Math.max(r.from, from,), Math.min(r.to, to,),);
    }
    return result;
  }
  /// @internal
  constructor(input, ranges,) {
    this.input = input;
    this.ranges = ranges;
    this.chunk = '';
    this.chunkOff = 0;
    this.chunk2 = '';
    this.chunk2Pos = 0;
    this.next = -1;
    this.token = nullToken;
    this.rangeIndex = 0;
    this.pos = this.chunkPos = ranges[0].from;
    this.range = ranges[0];
    this.end = ranges[ranges.length - 1].to;
    this.readNext();
  }
};
var TokenGroup = class {
  token(input, stack,) {
    let { parser, } = stack.p;
    readToken2(this.data, input, stack, this.id, parser.data, parser.tokenPrecTable,);
  }
  constructor(data, id2,) {
    this.data = data;
    this.id = id2;
  }
};
TokenGroup.prototype.contextual = TokenGroup.prototype.fallback = TokenGroup.prototype.extend = false;
var LocalTokenGroup = class {
  token(input, stack,) {
    let start = input.pos, skipped = 0;
    for (;;) {
      readToken2(this.data, input, stack, 0, this.data, this.precTable,);
      if (input.token.value > -1) break;
      if (this.elseToken == null) return;
      if (input.next < 0) break;
      input.advance();
      input.reset(input.pos, input.token,);
      skipped++;
    }
    if (skipped) {
      input.reset(start, input.token,);
      input.acceptToken(this.elseToken, skipped,);
    }
  }
  constructor(data, precTable, elseToken,) {
    this.precTable = precTable;
    this.elseToken = elseToken;
    this.data = typeof data == 'string' ? decodeArray(data,) : data;
  }
};
LocalTokenGroup.prototype.contextual = TokenGroup.prototype.fallback = TokenGroup.prototype.extend = false;
var ExternalTokenizer = class {
  /// Create a tokenizer. The first argument is the function that,
  /// given an input stream, scans for the types of tokens it
  /// recognizes at the stream's position, and calls
  /// [`acceptToken`](#lr.InputStream.acceptToken) when it finds
  /// one.
  constructor(token, options = {},) {
    this.token = token;
    this.contextual = !!options.contextual;
    this.fallback = !!options.fallback;
    this.extend = !!options.extend;
  }
};
function readToken2(data, input, stack, group, precTable, precOffset,) {
  let state = 0, groupMask = 1 << group, { dialect, } = stack.p.parser;
  scan: for (;;) {
    if ((groupMask & data[state]) == 0) break;
    let accEnd = data[state + 1];
    for (let i2 = state + 3; i2 < accEnd; i2 += 2) {
      if ((data[i2 + 1] & groupMask) > 0) {
        let term = data[i2];
        if (
          dialect.allows(term,) &&
          (input.token.value == -1 || input.token.value == term || overrides(term, input.token.value, precTable, precOffset,))
        ) {
          input.acceptToken(term,);
          break;
        }
      }
    }
    let next = input.next, low = 0, high = data[state + 2];
    if (input.next < 0 && high > low && data[accEnd + high * 3 - 3] == 65535 && data[accEnd + high * 3 - 3] == 65535) {
      state = data[accEnd + high * 3 - 1];
      continue scan;
    }
    for (; low < high;) {
      let mid = low + high >> 1;
      let index = accEnd + mid + (mid << 1);
      let from = data[index], to = data[index + 1] || 65536;
      if (next < from) high = mid;
      else if (next >= to) low = mid + 1;
      else {
        state = data[index + 2];
        input.advance();
        continue scan;
      }
    }
    break;
  }
}
function findOffset(data, start, term,) {
  for (let i2 = start, next; (next = data[i2]) != 65535; i2++) if (next == term) return i2 - start;
  return -1;
}
function overrides(token, prev, tableData, tableOffset,) {
  let iPrev = findOffset(tableData, tableOffset, prev,);
  return iPrev < 0 || findOffset(tableData, tableOffset, token,) < iPrev;
}
var verbose = typeof process_exports != 'undefined' && process_exports.env && /\bparse\b/.test(process_exports.env.LOG,);
var stackIDs = null;
var Safety;
(function (Safety2,) {
  Safety2[Safety2['Margin'] = 25] = 'Margin';
})(Safety || (Safety = {}),);
function cutAt(tree, pos, side,) {
  let cursor = tree.cursor(IterMode.IncludeAnonymous,);
  cursor.moveTo(pos,);
  for (;;) {
    if (!(side < 0 ? cursor.childBefore(pos,) : cursor.childAfter(pos,))) {
      for (;;) {
        if ((side < 0 ? cursor.to < pos : cursor.from > pos) && !cursor.type.isError) {
          return side < 0
            ? Math.max(0, Math.min(cursor.to - 1, pos - 25,),)
            : Math.min(tree.length, Math.max(cursor.from + 1, pos + 25,),);
        }
        if (side < 0 ? cursor.prevSibling() : cursor.nextSibling()) break;
        if (!cursor.parent()) return side < 0 ? 0 : tree.length;
      }
    }
  }
}
var FragmentCursor2 = class {
  nextFragment() {
    let fr = this.fragment = this.i == this.fragments.length ? null : this.fragments[this.i++];
    if (fr) {
      this.safeFrom = fr.openStart ? cutAt(fr.tree, fr.from + fr.offset, 1,) - fr.offset : fr.from;
      this.safeTo = fr.openEnd ? cutAt(fr.tree, fr.to + fr.offset, -1,) - fr.offset : fr.to;
      while (this.trees.length) {
        this.trees.pop();
        this.start.pop();
        this.index.pop();
      }
      this.trees.push(fr.tree,);
      this.start.push(-fr.offset,);
      this.index.push(0,);
      this.nextStart = this.safeFrom;
    } else {
      this.nextStart = 1e9;
    }
  }
  // `pos` must be >= any previously given `pos` for this cursor
  nodeAt(pos,) {
    if (pos < this.nextStart) return null;
    while (this.fragment && this.safeTo <= pos) this.nextFragment();
    if (!this.fragment) return null;
    for (;;) {
      let last = this.trees.length - 1;
      if (last < 0) {
        this.nextFragment();
        return null;
      }
      let top3 = this.trees[last], index = this.index[last];
      if (index == top3.children.length) {
        this.trees.pop();
        this.start.pop();
        this.index.pop();
        continue;
      }
      let next = top3.children[index];
      let start = this.start[last] + top3.positions[index];
      if (start > pos) {
        this.nextStart = start;
        return null;
      }
      if (next instanceof Tree) {
        if (start == pos) {
          if (start < this.safeFrom) return null;
          let end = start + next.length;
          if (end <= this.safeTo) {
            let lookAhead = next.prop(NodeProp.lookAhead,);
            if (!lookAhead || end + lookAhead < this.fragment.to) return next;
          }
        }
        this.index[last]++;
        if (start + next.length >= Math.max(this.safeFrom, pos,)) {
          this.trees.push(next,);
          this.start.push(start,);
          this.index.push(0,);
        }
      } else {
        this.index[last]++;
        this.nextStart = start + next.length;
      }
    }
  }
  constructor(fragments, nodeSet2,) {
    this.fragments = fragments;
    this.nodeSet = nodeSet2;
    this.i = 0;
    this.fragment = null;
    this.safeFrom = -1;
    this.safeTo = -1;
    this.trees = [];
    this.start = [];
    this.index = [];
    this.nextFragment();
  }
};
var TokenCache = class {
  getActions(stack,) {
    let actionIndex = 0;
    let main = null;
    let { parser, } = stack.p, { tokenizers, } = parser;
    let mask = parser.stateSlot(stack.state, 3,);
    let context = stack.curContext ? stack.curContext.hash : 0;
    let lookAhead = 0;
    for (let i2 = 0; i2 < tokenizers.length; i2++) {
      if ((1 << i2 & mask) == 0) continue;
      let tokenizer = tokenizers[i2], token = this.tokens[i2];
      if (main && !tokenizer.fallback) continue;
      if (tokenizer.contextual || token.start != stack.pos || token.mask != mask || token.context != context) {
        this.updateCachedToken(token, tokenizer, stack,);
        token.mask = mask;
        token.context = context;
      }
      if (token.lookAhead > token.end + 25) lookAhead = Math.max(token.lookAhead, lookAhead,);
      if (token.value != 0) {
        let startIndex = actionIndex;
        if (token.extended > -1) actionIndex = this.addActions(stack, token.extended, token.end, actionIndex,);
        actionIndex = this.addActions(stack, token.value, token.end, actionIndex,);
        if (!tokenizer.extend) {
          main = token;
          if (actionIndex > startIndex) break;
        }
      }
    }
    while (this.actions.length > actionIndex) this.actions.pop();
    if (lookAhead) stack.setLookAhead(lookAhead,);
    if (!main && stack.pos == this.stream.end) {
      main = new CachedToken();
      main.value = stack.p.parser.eofTerm;
      main.start = main.end = stack.pos;
      actionIndex = this.addActions(stack, main.value, main.end, actionIndex,);
    }
    this.mainToken = main;
    return this.actions;
  }
  getMainToken(stack,) {
    if (this.mainToken) return this.mainToken;
    let main = new CachedToken(), { pos, p, } = stack;
    main.start = pos;
    main.end = Math.min(pos + 1, p.stream.end,);
    main.value = pos == p.stream.end ? p.parser.eofTerm : 0;
    return main;
  }
  updateCachedToken(token, tokenizer, stack,) {
    let start = this.stream.clipPos(stack.pos,);
    tokenizer.token(this.stream.reset(start, token,), stack,);
    if (token.value > -1) {
      let { parser, } = stack.p;
      for (let i2 = 0; i2 < parser.specialized.length; i2++) {
        if (parser.specialized[i2] == token.value) {
          let result = parser.specializers[i2](this.stream.read(token.start, token.end,), stack,);
          if (result >= 0 && stack.p.parser.dialect.allows(result >> 1,)) {
            if ((result & 1) == 0) token.value = result >> 1;
            else token.extended = result >> 1;
            break;
          }
        }
      }
    } else {
      token.value = 0;
      token.end = this.stream.clipPos(start + 1,);
    }
  }
  putAction(action, token, end, index,) {
    for (let i2 = 0; i2 < index; i2 += 3) if (this.actions[i2] == action) return index;
    this.actions[index++] = action;
    this.actions[index++] = token;
    this.actions[index++] = end;
    return index;
  }
  addActions(stack, token, end, index,) {
    let { state, } = stack, { parser, } = stack.p, { data, } = parser;
    for (let set = 0; set < 2; set++) {
      for (let i2 = parser.stateSlot(state, set ? 2 : 1,);; i2 += 3) {
        if (data[i2] == 65535) {
          if (data[i2 + 1] == 1) {
            i2 = pair(data, i2 + 2,);
          } else {
            if (index == 0 && data[i2 + 1] == 2) index = this.putAction(pair(data, i2 + 2,), token, end, index,);
            break;
          }
        }
        if (data[i2] == token) index = this.putAction(pair(data, i2 + 1,), token, end, index,);
      }
    }
    return index;
  }
  constructor(parser, stream,) {
    this.stream = stream;
    this.tokens = [];
    this.mainToken = null;
    this.actions = [];
    this.tokens = parser.tokenizers.map((_,) => new CachedToken());
  }
};
var Rec;
(function (Rec2,) {
  Rec2[Rec2['Distance'] = 5] = 'Distance';
  Rec2[Rec2['MaxRemainingPerStep'] = 3] = 'MaxRemainingPerStep';
  Rec2[Rec2['MinBufferLengthPrune'] = 500] = 'MinBufferLengthPrune';
  Rec2[Rec2['ForceReduceLimit'] = 10] = 'ForceReduceLimit';
  Rec2[Rec2['CutDepth'] = 15e3] = 'CutDepth';
  Rec2[Rec2['CutTo'] = 9e3] = 'CutTo';
  Rec2[Rec2['MaxLeftAssociativeReductionCount'] = 300] = 'MaxLeftAssociativeReductionCount';
  Rec2[Rec2['MaxStackCount'] = 12] = 'MaxStackCount';
})(Rec || (Rec = {}),);
var Parse2 = class {
  get parsedPos() {
    return this.minStackPos;
  }
  // Move the parser forward. This will process all parse stacks at
  // `this.pos` and try to advance them to a further position. If no
  // stack for such a position is found, it'll start error-recovery.
  //
  // When the parse is finished, this will return a syntax tree. When
  // not, it returns `null`.
  advance() {
    let stacks = this.stacks, pos = this.minStackPos;
    let newStacks = this.stacks = [];
    let stopped, stoppedTokens;
    if (this.bigReductionCount > 300 && stacks.length == 1) {
      let [s,] = stacks;
      while (s.forceReduce() && s.stack.length && s.stack[s.stack.length - 2] >= this.lastBigReductionStart) {
      }
      this.bigReductionCount = this.lastBigReductionSize = 0;
    }
    for (let i2 = 0; i2 < stacks.length; i2++) {
      let stack = stacks[i2];
      for (;;) {
        this.tokens.mainToken = null;
        if (stack.pos > pos) {
          newStacks.push(stack,);
        } else if (this.advanceStack(stack, newStacks, stacks,)) {
          continue;
        } else {
          if (!stopped) {
            stopped = [];
            stoppedTokens = [];
          }
          stopped.push(stack,);
          let tok = this.tokens.getMainToken(stack,);
          stoppedTokens.push(tok.value, tok.end,);
        }
        break;
      }
    }
    if (!newStacks.length) {
      let finished = stopped && findFinished(stopped,);
      if (finished) return this.stackToTree(finished,);
      if (this.parser.strict) {
        if (verbose && stopped) {
          console.log('Stuck with token ' + (this.tokens.mainToken ? this.parser.getName(this.tokens.mainToken.value,) : 'none'),);
        }
        throw new SyntaxError('No parse at ' + pos,);
      }
      if (!this.recovering) this.recovering = 5;
    }
    if (this.recovering && stopped) {
      let finished1 = this.stoppedAt != null && stopped[0].pos > this.stoppedAt
        ? stopped[0]
        : this.runRecovery(stopped, stoppedTokens, newStacks,);
      if (finished1) return this.stackToTree(finished1.forceAll(),);
    }
    if (this.recovering) {
      let maxRemaining = this.recovering == 1 ? 1 : this.recovering * 3;
      if (newStacks.length > maxRemaining) {
        newStacks.sort((a, b,) => b.score - a.score);
        while (newStacks.length > maxRemaining) newStacks.pop();
      }
      if (newStacks.some((s,) => s.reducePos > pos)) this.recovering--;
    } else if (newStacks.length > 1) {
      outer: for (let i1 = 0; i1 < newStacks.length - 1; i1++) {
        let stack1 = newStacks[i1];
        for (let j = i1 + 1; j < newStacks.length; j++) {
          let other = newStacks[j];
          if (stack1.sameState(other,) || stack1.buffer.length > 500 && other.buffer.length > 500) {
            if ((stack1.score - other.score || stack1.buffer.length - other.buffer.length) > 0) {
              newStacks.splice(j--, 1,);
            } else {
              newStacks.splice(i1--, 1,);
              continue outer;
            }
          }
        }
      }
      if (newStacks.length > 12) newStacks.splice(12, newStacks.length - 12,);
    }
    this.minStackPos = newStacks[0].pos;
    for (let i2 = 1; i2 < newStacks.length; i2++) if (newStacks[i2].pos < this.minStackPos) this.minStackPos = newStacks[i2].pos;
    return null;
  }
  stopAt(pos,) {
    if (this.stoppedAt != null && this.stoppedAt < pos) throw new RangeError('Can\'t move stoppedAt forward',);
    this.stoppedAt = pos;
  }
  // Returns an updated version of the given stack, or null if the
  // stack can't advance normally. When `split` and `stacks` are
  // given, stacks split off by ambiguous operations will be pushed to
  // `split`, or added to `stacks` if they move `pos` forward.
  advanceStack(stack, stacks, split,) {
    let start = stack.pos, { parser, } = this;
    let base2 = verbose ? this.stackID(stack,) + ' -> ' : '';
    if (this.stoppedAt != null && start > this.stoppedAt) return stack.forceReduce() ? stack : null;
    if (this.fragments) {
      let strictCx = stack.curContext && stack.curContext.tracker.strict, cxHash = strictCx ? stack.curContext.hash : 0;
      for (let cached = this.fragments.nodeAt(start,); cached;) {
        let match = this.parser.nodeSet.types[cached.type.id] == cached.type ? parser.getGoto(stack.state, cached.type.id,) : -1;
        if (match > -1 && cached.length && (!strictCx || (cached.prop(NodeProp.contextHash,) || 0) == cxHash)) {
          stack.useNode(cached, match,);
          if (verbose) console.log(base2 + this.stackID(stack,) + ` (via reuse of ${parser.getName(cached.type.id,)})`,);
          return true;
        }
        if (!(cached instanceof Tree) || cached.children.length == 0 || cached.positions[0] > 0) break;
        let inner = cached.children[0];
        if (inner instanceof Tree && cached.positions[0] == 0) cached = inner;
        else break;
      }
    }
    let defaultReduce = parser.stateSlot(stack.state, 4,);
    if (defaultReduce > 0) {
      stack.reduce(defaultReduce,);
      if (verbose) console.log(base2 + this.stackID(stack,) + ` (via always-reduce ${parser.getName(defaultReduce & 65535,)})`,);
      return true;
    }
    if (stack.stack.length >= 15e3) {
      while (stack.stack.length > 9e3 && stack.forceReduce()) {
      }
    }
    let actions = this.tokens.getActions(stack,);
    for (let i2 = 0; i2 < actions.length;) {
      let action = actions[i2++], term = actions[i2++], end = actions[i2++];
      let last = i2 == actions.length || !split;
      let localStack = last ? stack : stack.split();
      localStack.apply(action, term, end,);
      if (verbose) {
        console.log(
          base2 + this.stackID(localStack,) +
            ` (via ${(action & 65536) == 0 ? 'shift' : `reduce of ${parser.getName(action & 65535,)}`} for ${
              parser.getName(term,)
            } @ ${start}${localStack == stack ? '' : ', split'})`,
        );
      }
      if (last) return true;
      else if (localStack.pos > start) stacks.push(localStack,);
      else split.push(localStack,);
    }
    return false;
  }
  // Advance a given stack forward as far as it will go. Returns the
  // (possibly updated) stack if it got stuck, or null if it moved
  // forward and was given to `pushStackDedup`.
  advanceFully(stack, newStacks,) {
    let pos = stack.pos;
    for (;;) {
      if (!this.advanceStack(stack, null, null,)) return false;
      if (stack.pos > pos) {
        pushStackDedup(stack, newStacks,);
        return true;
      }
    }
  }
  runRecovery(stacks, tokens, newStacks,) {
    let finished = null, restarted = false;
    for (let i2 = 0; i2 < stacks.length; i2++) {
      let stack = stacks[i2], token = tokens[i2 << 1], tokenEnd = tokens[(i2 << 1) + 1];
      let base2 = verbose ? this.stackID(stack,) + ' -> ' : '';
      if (stack.deadEnd) {
        if (restarted) continue;
        restarted = true;
        stack.restart();
        if (verbose) console.log(base2 + this.stackID(stack,) + ' (restarted)',);
        let done = this.advanceFully(stack, newStacks,);
        if (done) continue;
      }
      let force = stack.split(), forceBase = base2;
      for (let j = 0; force.forceReduce() && j < 10; j++) {
        if (verbose) console.log(forceBase + this.stackID(force,) + ' (via force-reduce)',);
        let done1 = this.advanceFully(force, newStacks,);
        if (done1) break;
        if (verbose) forceBase = this.stackID(force,) + ' -> ';
      }
      for (let insert2 of stack.recoverByInsert(token,)) {
        if (verbose) console.log(base2 + this.stackID(insert2,) + ' (via recover-insert)',);
        this.advanceFully(insert2, newStacks,);
      }
      if (this.stream.end > stack.pos) {
        if (tokenEnd == stack.pos) {
          tokenEnd++;
          token = 0;
        }
        stack.recoverByDelete(token, tokenEnd,);
        if (verbose) console.log(base2 + this.stackID(stack,) + ` (via recover-delete ${this.parser.getName(token,)})`,);
        pushStackDedup(stack, newStacks,);
      } else if (!finished || finished.score < stack.score) {
        finished = stack;
      }
    }
    return finished;
  }
  // Convert the stack's buffer to a syntax tree.
  stackToTree(stack,) {
    stack.close();
    return Tree.build({
      buffer: StackBufferCursor.create(stack,),
      nodeSet: this.parser.nodeSet,
      topID: this.topTerm,
      maxBufferLength: this.parser.bufferLength,
      reused: this.reused,
      start: this.ranges[0].from,
      length: stack.pos - this.ranges[0].from,
      minRepeatType: this.parser.minRepeatTerm,
    },);
  }
  stackID(stack,) {
    let id2 = (stackIDs || (stackIDs = /* @__PURE__ */ new WeakMap())).get(stack,);
    if (!id2) stackIDs.set(stack, id2 = String.fromCodePoint(this.nextStackID++,),);
    return id2 + stack;
  }
  constructor(parser, input, fragments, ranges,) {
    this.parser = parser;
    this.input = input;
    this.ranges = ranges;
    this.recovering = 0;
    this.nextStackID = 9812;
    this.minStackPos = 0;
    this.reused = [];
    this.stoppedAt = null;
    this.lastBigReductionStart = -1;
    this.lastBigReductionSize = 0;
    this.bigReductionCount = 0;
    this.stream = new InputStream(input, ranges,);
    this.tokens = new TokenCache(parser, this.stream,);
    this.topTerm = parser.top[1];
    let { from, } = ranges[0];
    this.stacks = [Stack.start(this, parser.top[0], from,),];
    this.fragments = fragments.length && this.stream.end - from > parser.bufferLength * 4
      ? new FragmentCursor2(fragments, parser.nodeSet,)
      : null;
  }
};
function pushStackDedup(stack, newStacks,) {
  for (let i2 = 0; i2 < newStacks.length; i2++) {
    let other = newStacks[i2];
    if (other.pos == stack.pos && other.sameState(stack,)) {
      if (newStacks[i2].score < stack.score) newStacks[i2] = stack;
      return;
    }
  }
  newStacks.push(stack,);
}
var Dialect = class {
  allows(term,) {
    return !this.disabled || this.disabled[term] == 0;
  }
  constructor(source, flags, disabled,) {
    this.source = source;
    this.flags = flags;
    this.disabled = disabled;
  }
};
var id = (x,) => x;
var ContextTracker = class {
  /// Define a context tracker.
  constructor(spec,) {
    this.start = spec.start;
    this.shift = spec.shift || id;
    this.reduce = spec.reduce || id;
    this.reuse = spec.reuse || id;
    this.hash = spec.hash || (() => 0);
    this.strict = spec.strict !== false;
  }
};
var LRParser = class extends Parser {
  createParse(input, fragments, ranges,) {
    let parse = new Parse2(this, input, fragments, ranges,);
    for (let w of this.wrappers) parse = w(parse, input, fragments, ranges,);
    return parse;
  }
  /// Get a goto table entry @internal
  getGoto(state, term, loose = false,) {
    let table = this.goto;
    if (term >= table[0]) return -1;
    for (let pos = table[term + 1];;) {
      let groupTag = table[pos++], last = groupTag & 1;
      let target = table[pos++];
      if (last && loose) return target;
      for (let end = pos + (groupTag >> 1); pos < end; pos++) if (table[pos] == state) return target;
      if (last) return -1;
    }
  }
  /// Check if this state has an action for a given terminal @internal
  hasAction(state, terminal,) {
    let data = this.data;
    for (let set = 0; set < 2; set++) {
      for (let i2 = this.stateSlot(state, set ? 2 : 1,), next;; i2 += 3) {
        if ((next = data[i2]) == 65535) {
          if (data[i2 + 1] == 1) next = data[i2 = pair(data, i2 + 2,)];
          else if (data[i2 + 1] == 2) return pair(data, i2 + 2,);
          else break;
        }
        if (next == terminal || next == 0) return pair(data, i2 + 1,);
      }
    }
    return 0;
  }
  /// @internal
  stateSlot(state, slot,) {
    return this.states[state * 6 + slot];
  }
  /// @internal
  stateFlag(state, flag,) {
    return (this.stateSlot(state, 0,) & flag) > 0;
  }
  /// @internal
  validAction(state, action,) {
    return !!this.allActions(state, (a,) => a == action ? true : null,);
  }
  /// @internal
  allActions(state, action,) {
    let deflt = this.stateSlot(state, 4,);
    let result = deflt ? action(deflt,) : void 0;
    for (let i2 = this.stateSlot(state, 1,); result == null; i2 += 3) {
      if (this.data[i2] == 65535) {
        if (this.data[i2 + 1] == 1) i2 = pair(this.data, i2 + 2,);
        else break;
      }
      result = action(pair(this.data, i2 + 1,),);
    }
    return result;
  }
  /// Get the states that can follow this one through shift actions or
  /// goto jumps. @internal
  nextStates(state,) {
    let result = [];
    for (let i2 = this.stateSlot(state, 1,);; i2 += 3) {
      if (this.data[i2] == 65535) {
        if (this.data[i2 + 1] == 1) i2 = pair(this.data, i2 + 2,);
        else break;
      }
      if ((this.data[i2 + 2] & 65536 >> 16) == 0) {
        let value = this.data[i2 + 1];
        if (!result.some((v, i22,) => i22 & 1 && v == value)) result.push(this.data[i2], value,);
      }
    }
    return result;
  }
  /// Configure the parser. Returns a new parser instance that has the
  /// given settings modified. Settings not provided in `config` are
  /// kept from the original parser.
  configure(config,) {
    let copy = Object.assign(Object.create(LRParser.prototype,), this,);
    if (config.props) copy.nodeSet = this.nodeSet.extend(...config.props,);
    if (config.top) {
      let info = this.topRules[config.top];
      if (!info) throw new RangeError(`Invalid top rule name ${config.top}`,);
      copy.top = info;
    }
    if (config.tokenizers) {
      copy.tokenizers = this.tokenizers.map((t2,) => {
        let found = config.tokenizers.find((r,) => r.from == t2);
        return found ? found.to : t2;
      },);
    }
    if (config.specializers) {
      copy.specializers = this.specializers.slice();
      copy.specializerSpecs = this.specializerSpecs.map((s, i2,) => {
        let found = config.specializers.find((r,) => r.from == s.external);
        if (!found) return s;
        let spec = Object.assign(Object.assign({}, s,), { external: found.to, },);
        copy.specializers[i2] = getSpecializer(spec,);
        return spec;
      },);
    }
    if (config.contextTracker) copy.context = config.contextTracker;
    if (config.dialect) copy.dialect = this.parseDialect(config.dialect,);
    if (config.strict != null) copy.strict = config.strict;
    if (config.wrap) copy.wrappers = copy.wrappers.concat(config.wrap,);
    if (config.bufferLength != null) copy.bufferLength = config.bufferLength;
    return copy;
  }
  /// Tells you whether any [parse wrappers](#lr.ParserConfig.wrap)
  /// are registered for this parser.
  hasWrappers() {
    return this.wrappers.length > 0;
  }
  /// Returns the name associated with a given term. This will only
  /// work for all terms when the parser was generated with the
  /// `--names` option. By default, only the names of tagged terms are
  /// stored.
  getName(term,) {
    return this.termNames ? this.termNames[term] : String(term <= this.maxNode && this.nodeSet.types[term].name || term,);
  }
  /// The eof term id is always allocated directly after the node
  /// types. @internal
  get eofTerm() {
    return this.maxNode + 1;
  }
  /// The type of top node produced by the parser.
  get topNode() {
    return this.nodeSet.types[this.top[1]];
  }
  /// @internal
  dynamicPrecedence(term,) {
    let prec2 = this.dynamicPrecedences;
    return prec2 == null ? 0 : prec2[term] || 0;
  }
  /// @internal
  parseDialect(dialect,) {
    let values = Object.keys(this.dialects,), flags = values.map(() => false);
    if (dialect) {
      for (let part of dialect.split(' ',)) {
        let id2 = values.indexOf(part,);
        if (id2 >= 0) flags[id2] = true;
      }
    }
    let disabled = null;
    for (let i2 = 0; i2 < values.length; i2++) {
      if (!flags[i2]) {
        for (
          let j = this.dialects[values[i2]], id21; (id21 = this.data[j++]) != 65535;
        ) (disabled || (disabled = new Uint8Array(this.maxTerm + 1,)))[id21] = 1;
      }
    }
    return new Dialect(dialect, flags, disabled,);
  }
  /// Used by the output of the parser generator. Not available to
  /// user code. @hide
  static deserialize(spec,) {
    return new LRParser(spec,);
  }
  /// @internal
  constructor(spec,) {
    super();
    this.wrappers = [];
    if (spec.version != 14) throw new RangeError(`Parser version (${spec.version}) doesn't match runtime version (${14})`,);
    let nodeNames = spec.nodeNames.split(' ',);
    this.minRepeatTerm = nodeNames.length;
    for (let i2 = 0; i2 < spec.repeatNodeCount; i2++) nodeNames.push('',);
    let topTerms = Object.keys(spec.topRules,).map((r,) => spec.topRules[r][1]);
    let nodeProps = [];
    for (let i1 = 0; i1 < nodeNames.length; i1++) nodeProps.push([],);
    function setProp(nodeID, prop, value,) {
      nodeProps[nodeID].push([prop, prop.deserialize(String(value,),),],);
    }
    if (spec.nodeProps) {
      for (let propSpec of spec.nodeProps) {
        let prop = propSpec[0];
        if (typeof prop == 'string') prop = NodeProp[prop];
        for (let i2 = 1; i2 < propSpec.length;) {
          let next = propSpec[i2++];
          if (next >= 0) {
            setProp(next, prop, propSpec[i2++],);
          } else {
            let value = propSpec[i2 + -next];
            for (let j = -next; j > 0; j--) setProp(propSpec[i2++], prop, value,);
            i2++;
          }
        }
      }
    }
    this.nodeSet = new NodeSet(
      nodeNames.map((name2, i2,) =>
        NodeType.define({
          name: i2 >= this.minRepeatTerm ? void 0 : name2,
          id: i2,
          props: nodeProps[i2],
          top: topTerms.indexOf(i2,) > -1,
          error: i2 == 0,
          skipped: spec.skippedNodes && spec.skippedNodes.indexOf(i2,) > -1,
        },)
      ),
    );
    if (spec.propSources) this.nodeSet = this.nodeSet.extend(...spec.propSources,);
    this.strict = false;
    this.bufferLength = DefaultBufferLength;
    let tokenArray = decodeArray(spec.tokenData,);
    this.context = spec.context;
    this.specializerSpecs = spec.specialized || [];
    this.specialized = new Uint16Array(this.specializerSpecs.length,);
    for (let i3 = 0; i3 < this.specializerSpecs.length; i3++) this.specialized[i3] = this.specializerSpecs[i3].term;
    this.specializers = this.specializerSpecs.map(getSpecializer,);
    this.states = decodeArray(spec.states, Uint32Array,);
    this.data = decodeArray(spec.stateData,);
    this.goto = decodeArray(spec.goto,);
    this.maxTerm = spec.maxTerm;
    this.tokenizers = spec.tokenizers.map((value,) => typeof value == 'number' ? new TokenGroup(tokenArray, value,) : value);
    this.topRules = spec.topRules;
    this.dialects = spec.dialects || {};
    this.dynamicPrecedences = spec.dynamicPrecedences || null;
    this.tokenPrecTable = spec.tokenPrec;
    this.termNames = spec.termNames || null;
    this.maxNode = this.nodeSet.types.length - 1;
    this.dialect = this.parseDialect();
    this.top = this.topRules[Object.keys(this.topRules,)[0]];
  }
};
function pair(data, off,) {
  return data[off] | data[off + 1] << 16;
}
function findFinished(stacks,) {
  let best = null;
  for (let stack of stacks) {
    let stopped = stack.p.stoppedAt;
    if (
      (stack.pos == stack.p.stream.end || stopped != null && stack.pos > stopped) && stack.p.parser.stateFlag(stack.state, 2,) &&
      (!best || best.score < stack.score)
    ) best = stack;
  }
  return best;
}
function getSpecializer(spec,) {
  if (spec.external) {
    let mask = spec.extend ? 1 : 0;
    return (value, stack,) => spec.external(value, stack,) << 1 | mask;
  }
  return spec.get;
}

export {
  Annotation,
  bracketMatching,
  bracketMatchingHandle,
  ChangeDesc,
  ChangeSet,
  CharCategory,
  codePointAt,
  codePointSize,
  combineConfig,
  ContextTracker,
  continuedIndent,
  countColumn,
  Decoration,
  defineLanguageFacet,
  delimitedIndent,
  Direction,
  EditorSelection,
  EditorState,
  EditorView,
  ExternalTokenizer,
  Facet,
  findClusterBreak,
  flatIndent,
  foldInside,
  foldNodeProp,
  foldService,
  fromCodePoint,
  getIndentation,
  getIndentUnit,
  highlightActiveLine,
  highlightSpecialChars,
  HighlightStyle,
  highlightTree,
  IndentContext,
  indentNodeProp,
  indentString,
  indentUnit,
  IterMode,
  keymap,
  Language,
  languageDataProp,
  LanguageDescription,
  LanguageSupport,
  lineNumbers,
  LocalTokenGroup,
  LRLanguage,
  LRParser,
  MapMode,
  matchBrackets,
  NodeProp,
  NodeSet,
  NodeType,
  NodeWeakMap,
  ParseContext,
  parseMixed,
  Parser,
  Prec,
  RangeSet,
  RangeValue,
  StateEffect,
  StateField,
  StreamLanguage,
  styleTags,
  sublanguageProp,
  syntaxHighlighting,
  syntaxTree,
  Tag,
  tags,
  Text,
  Transaction,
  Tree,
  ViewPlugin,
  WidgetType,
};
