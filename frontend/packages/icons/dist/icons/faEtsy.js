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

// src/node_modules/@fortawesome/free-brands-svg-icons/faEtsy.js
var require_faEtsy = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faEtsy.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "etsy";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "f2d7";
    var svgPathData = "M384 348c-1.8 10.7-13.8 110-15.5 132-117.9-4.3-219.9-4.7-368.5 0l0-25.5c45.5-8.9 60.6-8 61-35.2 1.8-72.3 3.5-244.1 0-322-1-28.5-12.1-26.8-61-36L0 35.8c73.9 2.4 255.9 8.6 363-3.8-3.5 38.2-7.8 126.5-7.8 126.5l-23.2 0C320.9 115.7 313.2 68 277.3 68l-137 0c-10.2 0-10.7 3.5-10.7 9.8l0 163.8c58 .5 88.5-2.5 88.5-2.5 29.8-1 27.6-8.5 40.7-65.3l25.8 0c-4.4 101.4-3.9 61.8-1.8 160.3L257 334c-9.2-40.1-9.1-61-39.5-61.5 0 0-21.5-2-88-2l0 139c0 26 14.3 38.3 44.3 38.3l89.3 0c63.6 0 66.6-25 98.7-99.8l22.2 0z";
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
    exports.faEtsy = exports.definition;
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

// src/_entries/faEtsy.js
var import_faEtsy = __toESM(require_faEtsy());
var export_faEtsy = import_faEtsy.definition;
export {
  export_faEtsy as faEtsy
};
