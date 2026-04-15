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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPaintbrushPencil.js
var require_faPaintbrushPencil = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPaintbrushPencil.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "paintbrush-pencil";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e206";
    var svgPathData = "M559.6 95.6c21.9-21.9 21.9-57.3 0-79.2s-57.3-21.9-79.2 0L227.7 269.1l79.2 79.2L559.6 95.6zM205 291.8c-9.3-2.5-19-3.8-29-3.8c-61.9 0-112 50.1-112 112c0 3.9 .2 7.8 .6 11.6C66.4 429.1 54.4 448 36.8 448L32 448c-17.7 0-32 14.3-32 32s14.3 32 32 32l144 0c61.9 0 112-50.1 112-112c0-10-1.3-19.8-3.8-29l.1-.1-79.2-79.2-.1 .1zm93.4-138.7L164.7 19.3c-25-25-65.5-25-90.5 0L50.7 42.7c-25 25-25 65.5 0 90.5L173.5 256c.8 0 1.7 0 2.5 0c6.2 0 12.4 .4 18.4 1.2L298.5 153.1zM320 402.5l64.6 64.6c6.7 6.7 15.1 11.6 24.2 14.2l104 29.7c8.4 2.4 17.4 .1 23.6-6.1s8.5-15.2 6.1-23.6l-29.7-104c-2.6-9.2-7.5-17.5-14.2-24.2l-75.6-75.6L318.8 381.6c.8 6 1.2 12.2 1.2 18.4c0 .8 0 1.7 0 2.5z";
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
    exports.faPaintbrushPencil = exports.definition;
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

// src/_entries/faPaintbrushPencil.js
var import_faPaintbrushPencil = __toESM(require_faPaintbrushPencil());
var export_faPaintbrushPencil = import_faPaintbrushPencil.definition;
export {
  export_faPaintbrushPencil as faPaintbrushPencil
};
