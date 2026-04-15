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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faGif.js
var require_faGif = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faGif.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "gif";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e190";
    var svgPathData = "M0 96C0 60.7 28.7 32 64 32l448 0c35.3 0 64 28.7 64 64l0 320c0 35.3-28.7 64-64 64L64 480c-35.3 0-64-28.7-64-64L0 96zm296 64c-13.3 0-24 10.7-24 24l0 144c0 13.3 10.7 24 24 24s24-10.7 24-24l0-144c0-13.3-10.7-24-24-24zm56 24l0 80 0 64c0 13.3 10.7 24 24 24s24-10.7 24-24l0-40 40 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-40 0 0-32 64 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-88 0c-13.3 0-24 10.7-24 24zM128 256c0-26.5 21.5-48 48-48c8 0 15.4 1.9 22 5.3c11.8 6.1 26.3 1.5 32.3-10.3s1.5-26.3-10.3-32.3c-13.2-6.8-28.2-10.7-44-10.7c-53 0-96 43-96 96s43 96 96 96c19.6 0 37.5-6.1 52.8-15.8c7-4.4 11.2-12.1 11.2-20.3l0-51.9c0-13.3-10.7-24-24-24l-32 0c-13.3 0-24 10.7-24 24s10.7 24 24 24l8 0 0 13.1c-5.3 1.9-10.6 2.9-16 2.9c-26.5 0-48-21.5-48-48z";
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
    exports.faGif = exports.definition;
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

// src/_entries/faGif.js
var import_faGif = __toESM(require_faGif());
var export_faGif = import_faGif.definition;
export {
  export_faGif as faGif
};
