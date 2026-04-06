var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));

// src/node_modules/@fortawesome/free-solid-svg-icons/faClosedCaptioningSlash.js
var require_faClosedCaptioningSlash = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faClosedCaptioningSlash.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "closed-captioning-slash";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e135";
    var svgPathData = "M7-24.9c9.4-9.4 24.6-9.4 33.9 0L129.8 64 480 64c35.3 0 64 28.7 64 64l0 256c0 25.5-14.9 47.4-36.4 57.8L569 503.1c9.4 9.4 9.4 24.6 0 33.9s-24.6 9.4-33.9 0L7 9.1C-2.3-.3-2.3-15.5 7-24.9zm345 311l0-70.2c0-4.4 3.6-8 8-8l32 0c4.4 0 8 3.6 8 8 0 13.3 10.7 24 24 24s24-10.7 24-24c0-30.9-25.1-56-56-56l-32 0c-30.9 0-56 25.1-56 56l0 22.2 48 48zm61.5 61.5c20.2-8.4 34.5-28.4 34.5-51.7 0-13.3-10.7-24-24-24s-24 10.7-24 24c0 4.4-3.6 8-8 8l-22.2 0 43.7 43.7zM130.3 200.1c-1.5 5-2.3 10.4-2.3 15.9l0 80c0 30.9 25.1 56 56 56l32 0c18.1 0 34.1-8.5 44.4-21.8L378.2 448 96 448c-35.3 0-64-28.7-64-64l0-256c0-7.8 1.4-15.3 4-22.2l94.3 94.3zm93.8 93.8c-.1 .7-.1 1.4-.1 2.1 0 4.4-3.6 8-8 8l-32 0c-4.4 0-8-3.6-8-8l0-50.2 48.1 48.1z";
    exports.definition = {
      prefix,
      iconName,
      icon: [
        width,
        height,
        aliases,
        unicode,
        svgPathData
      ]
    };
    exports.faClosedCaptioningSlash = exports.definition;
    exports.prefix = prefix;
    exports.iconName = iconName;
    exports.width = width;
    exports.height = height;
    exports.ligatures = aliases;
    exports.unicode = unicode;
    exports.svgPathData = svgPathData;
    exports.aliases = aliases;
  }
});

// src/_entries/faClosedCaptioningSlash.js
var import_faClosedCaptioningSlash = __toESM(require_faClosedCaptioningSlash());
var export_faClosedCaptioningSlash = import_faClosedCaptioningSlash.definition;
export {
  export_faClosedCaptioningSlash as faClosedCaptioningSlash
};
