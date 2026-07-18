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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faUserAlien.js
var require_faUserAlien = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faUserAlien.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "user-alien";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e04a";
    var svgPathData = "M64 167C64 74.8 135.6 0 224 0s160 74.8 160 167c0 101-106.6 185-139.2 208.3c-6.1 4.3-13.4 6.5-20.8 6.5s-14.7-2.1-20.8-6.5C170.6 352 64 268.1 64 167zm244.6 9c-37.9 0-68.6 30.7-68.6 68.6c0 6.3 5.1 11.4 11.4 11.4l16 0c37.9 0 68.6-30.7 68.6-68.6c0-6.3-5.1-11.4-11.4-11.4l-16 0zM208 244.6c0-37.9-30.7-68.6-68.6-68.6l-16 0c-6.3 0-11.4 5.1-11.4 11.4c0 37.9 30.7 68.6 68.6 68.6l16 0c6.3 0 11.4-5.1 11.4-11.4zM0 486.4C0 415 55.7 356.6 126.1 352.3c23.8 23.2 46.1 40.2 58.5 49.1c12 8.6 25.9 12.4 39.4 12.4s27.4-3.8 39.4-12.4c12.4-8.8 34.7-25.8 58.5-49.1C392.3 356.6 448 415 448 486.4c0 14.1-11.5 25.6-25.6 25.6L25.6 512C11.5 512 0 500.5 0 486.4z";
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
    exports.faUserAlien = exports.definition;
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

// src/_entries/faUserAlien.js
var import_faUserAlien = __toESM(require_faUserAlien());
var export_faUserAlien = import_faUserAlien.definition;
export {
  export_faUserAlien as faUserAlien
};
