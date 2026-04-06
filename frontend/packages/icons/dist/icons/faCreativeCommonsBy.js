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

// src/node_modules/@fortawesome/free-brands-svg-icons/faCreativeCommonsBy.js
var require_faCreativeCommonsBy = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faCreativeCommonsBy.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "creative-commons-by";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f4e7";
    var svgPathData = "M322.9 194.4l0 101.4-28.3 0 0 120.5-77.1 0 0-120.4-28.3 0 0-101.5c0-4.4 1.6-8.2 4.6-11.3 3.1-3.1 6.9-4.7 11.3-4.7l101.9 0c4.1 0 7.8 1.6 11.1 4.7 3.1 3.2 4.8 6.9 4.8 11.3zm-32.5-64.5a34.5 34.5 0 1 1 -69 .8 34.5 34.5 0 1 1 69-.8zM255.6 8C397.4 8 504 118.1 504 256 504 403.1 385.5 504 255.6 504 121.6 504 8 394.5 8 256 8 123.1 112.7 8 255.6 8zm.8 44.7c-118.2 0-203.7 97.9-203.7 203.3 0 109.8 91.2 202.8 203.7 202.8 103.2 0 202.8-81.1 202.8-202.8 .1-113.8-90.2-203.3-202.8-203.3z";
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
    exports.faCreativeCommonsBy = exports.definition;
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

// src/_entries/faCreativeCommonsBy.js
var import_faCreativeCommonsBy = __toESM(require_faCreativeCommonsBy());
var export_faCreativeCommonsBy = import_faCreativeCommonsBy.definition;
export {
  export_faCreativeCommonsBy as faCreativeCommonsBy
};
