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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBowlHot.js
var require_faBowlHot = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBowlHot.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "bowl-hot";
    var width = 512;
    var height = 512;
    var aliases = ["soup"];
    var unicode = "f823";
    var svgPathData = "M152 0c-13.3 0-24 10.7-24 24l0 8c0 32.7 15.4 63.6 41.6 83.2C183.7 125.8 192 142.4 192 160l0 8c0 13.3 10.7 24 24 24s24-10.7 24-24l0-8c0-32.7-15.4-63.6-41.6-83.2C184.3 66.2 176 49.6 176 32l0-8c0-13.3-10.7-24-24-24zM33.6 224C15.4 224 .9 239.3 2 257.5c5.4 91.7 59.7 169.7 136.4 209.7l1.2 9.7c2.5 20 19.5 35 39.7 35l153.4 0c20.2 0 37.2-15 39.7-35l1.2-9.7c76.8-40 131-118 136.4-209.7c1.1-18.2-13.4-33.5-31.6-33.5L33.6 224zM304 24c0-13.3-10.7-24-24-24s-24 10.7-24 24l0 8c0 32.7 15.4 63.6 41.6 83.2C311.7 125.8 320 142.4 320 160l0 8c0 13.3 10.7 24 24 24s24-10.7 24-24l0-8c0-32.7-15.4-63.6-41.6-83.2C312.3 66.2 304 49.6 304 32l0-8z";
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
    exports.faBowlHot = exports.definition;
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

// src/_entries/faSoup.js
var import_faBowlHot = __toESM(require_faBowlHot());
var export_faSoup = import_faBowlHot.definition;
export {
  export_faSoup as faSoup
};
