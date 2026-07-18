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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faStars.js
var require_faStars = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faStars.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "stars";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f762";
    var svgPathData = "M325.8 152.3c1.3 4.6 5.5 7.7 10.2 7.7s8.9-3.1 10.2-7.7L360 104l48.3-13.8c4.6-1.3 7.7-5.5 7.7-10.2s-3.1-8.9-7.7-10.2L360 56 346.2 7.7C344.9 3.1 340.7 0 336 0s-8.9 3.1-10.2 7.7L312 56 263.7 69.8c-4.6 1.3-7.7 5.5-7.7 10.2s3.1 8.9 7.7 10.2L312 104l13.8 48.3zm-112.4 5.1c-8.8-17.9-34.3-17.9-43.1 0l-46.3 94L20.5 266.5C.9 269.3-7 293.5 7.2 307.4l74.9 73.2L64.5 483.9c-3.4 19.6 17.2 34.6 34.8 25.3l92.6-48.8 92.6 48.8c17.6 9.3 38.2-5.7 34.8-25.3L301.6 380.6l74.9-73.2c14.2-13.9 6.4-38.1-13.3-40.9L259.7 251.4l-46.3-94zm215.4 85.8l11 38.6c1 3.6 4.4 6.2 8.2 6.2s7.1-2.5 8.2-6.2l11-38.6 38.6-11c3.6-1 6.2-4.4 6.2-8.2s-2.5-7.1-6.2-8.2l-38.6-11-11-38.6c-1-3.6-4.4-6.2-8.2-6.2s-7.1 2.5-8.2 6.2l-11 38.6-38.6 11c-3.6 1-6.2 4.4-6.2 8.2s2.5 7.1 6.2 8.2l38.6 11z";
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
    exports.faStars = exports.definition;
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

// src/_entries/faStars.js
var import_faStars = __toESM(require_faStars());
var export_faStars = import_faStars.definition;
export {
  export_faStars as faStars
};
