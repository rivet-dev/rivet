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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faImageSlash.js
var require_faImageSlash = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faImageSlash.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "image-slash";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e1b7";
    var svgPathData = "M38.8 5.1C28.4-3.1 13.3-1.2 5.1 9.2S-1.2 34.7 9.2 42.9l592 464c10.4 8.2 25.5 6.3 33.7-4.1s6.3-25.5-4.1-33.7l-55.5-43.5c.5-3.1 .7-6.3 .7-9.6l0-320c0-35.3-28.7-64-64-64L128 32c-14.4 0-27.8 4.8-38.5 12.9L38.8 5.1zm120 94.1c5.3-2 11.1-3.2 17.2-3.2c26.5 0 48 21.5 48 48c0 2-.1 4-.4 6L158.8 99.2zM328.1 231.9l20-29.4c4.5-6.6 11.9-10.5 19.8-10.5s15.4 3.9 19.8 10.5L499.4 366.1 328.1 231.9zM221.7 272.5c.3-.4 .6-.7 1-1.1L64 146.4 64 416c0 35.3 28.7 64 64 64l359.4 0-81.2-64L320 416l-72 0-96 0c-9.3 0-17.8-5.4-21.8-13.9s-2.6-18.5 3.5-25.6l88-104z";
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
    exports.faImageSlash = exports.definition;
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

// src/_entries/faImageSlash.js
var import_faImageSlash = __toESM(require_faImageSlash());
var export_faImageSlash = import_faImageSlash.definition;
export {
  export_faImageSlash as faImageSlash
};
