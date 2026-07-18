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

// src/node_modules/@fortawesome/free-brands-svg-icons/faCreativeCommonsPdAlt.js
var require_faCreativeCommonsPdAlt = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faCreativeCommonsPdAlt.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "creative-commons-pd-alt";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f4ed";
    var svgPathData = "M255.6 8C112.7 8 8 123.1 8 256 8 394.5 121.6 504 255.6 504 385.5 504 504 403.1 504 256 504 118.1 397.4 8 255.6 8zm.8 450.8c-112.5 0-203.7-93-203.7-202.8 0-105.4 85.5-203.3 203.7-203.3 112.6 0 202.9 89.5 202.8 203.3 0 121.7-99.6 202.8-202.8 202.8zM324.7 186l-53.2 0 0 137.2 53.2 0c21.4 0 70-5.1 70-68.6 0-63.4-48.6-68.6-70-68.6zm.8 108.5l-19.9 0 0-79.7 19.4-.1c3.8 0 35-2.1 35 39.9 0 24.6-10.5 39.9-34.5 39.9zM211.7 186l-68.2 0 0 137.3 34.6 0 0-44.3 27 0c54.1 0 57.1-37.5 57.1-46.5 0-31-16.8-46.5-50.5-46.5zm-4.9 67.3l-29.2 0 0-41.6 28.3 0c30.9 0 28.8 41.6 .9 41.6z";
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
    exports.faCreativeCommonsPdAlt = exports.definition;
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

// src/_entries/faCreativeCommonsPdAlt.js
var import_faCreativeCommonsPdAlt = __toESM(require_faCreativeCommonsPdAlt());
var export_faCreativeCommonsPdAlt = import_faCreativeCommonsPdAlt.definition;
export {
  export_faCreativeCommonsPdAlt as faCreativeCommonsPdAlt
};
