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

// src/node_modules/@fortawesome/free-solid-svg-icons/faBookSkull.js
var require_faBookSkull = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faBookSkull.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "book-skull";
    var width = 448;
    var height = 512;
    var aliases = ["book-dead"];
    var unicode = "f6b7";
    var svgPathData = "M96 512l320 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l0-66.7c18.6-6.6 32-24.4 32-45.3l0-288c0-26.5-21.5-48-48-48L96 0C43 0 0 43 0 96L0 416c0 53 43 96 96 96zM64 416c0-17.7 14.3-32 32-32l256 0 0 64-256 0c-17.7 0-32-14.3-32-32zM272 163.2l0 12.8c0 8.8-7.2 16-16 16l-64 0c-8.8 0-16-7.2-16-16l0-12.8c-19.4-11.7-32-30.3-32-51.2 0-35.3 35.8-64 80-64s80 28.7 80 64c0 20.9-12.6 39.5-32 51.2zM208 112a16 16 0 1 0 -32 0 16 16 0 1 0 32 0zm48 16a16 16 0 1 0 0-32 16 16 0 1 0 0 32zm74.2 95.7c4.6 10.1 .1 21.9-9.9 26.5l-47.9 21.8 47.9 21.8c10.1 4.6 14.5 16.4 9.9 26.5s-16.4 14.5-26.5 9.9L224 294 144.3 330.2c-10.1 4.6-21.9 .1-26.5-9.9s-.1-21.9 9.9-26.5l47.9-21.8-47.9-21.8c-10.1-4.6-14.5-16.4-9.9-26.5s16.4-14.5 26.5-9.9L224 250 303.7 213.8c10.1-4.6 21.9-.1 26.5 9.9z";
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
    exports.faBookSkull = exports.definition;
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

// src/_entries/faBookSkull.js
var import_faBookSkull = __toESM(require_faBookSkull());
var export_faBookSkull = import_faBookSkull.definition;
export {
  export_faBookSkull as faBookSkull
};
