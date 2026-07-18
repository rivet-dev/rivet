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

// src/node_modules/@fortawesome/free-brands-svg-icons/faBuyNLarge.js
var require_faBuyNLarge = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faBuyNLarge.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "buy-n-large";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f8a6";
    var svgPathData = "M288.2 32C133.5 32 8 132.3 8 256S133.5 480 288.2 480 568.4 379.7 568.4 256 442.9 32 288.2 32zM202.8 389.2l-138.5 1.4 77.2-290.7 133.4 0c63.2 0 84.9 28.6 78 72.8-.4 2.3-.9 4.6-1.5 6.9-5.6-1.3-11.4-1.9-17.2-1.9-42.3 0-76.7 33.9-76.7 75.7 0 37.1 27.1 68 62.9 74.5-18.2 37.2-56.2 60.9-117.7 61.5zM358.2 207.1l32 0-22.2 90.3-35.4 0-11.2-35.6-7.8 35.6-37.8 0 26.6-90.3 31.3 0 15 36.8 9.4-36.8zM504.1 389.2l-197.1 0 15.8-61.2c3.8 .6 7.6 .8 11.5 .8 42.3 0 76.7-33.9 76.7-75.6 0-32.6-21-60.5-50.4-71.1l21.3-82.4 92.5 0-53 205.4 103.9 0-21.2 84.1zM211.9 269.4l-24.7 0-13.8 56.5 24.7 0c16.1 0 32.1-3.2 37.9-26.6 5.6-22.3-8-29.8-24.1-29.8zM233.2 170l-21.3 0-11.7 47.7 21.4 0c18 0 35.4-14.6 39.2-30.1 4.6-18.9-9.5-17.6-27.6-17.6z";
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
    exports.faBuyNLarge = exports.definition;
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

// src/_entries/faBuyNLarge.js
var import_faBuyNLarge = __toESM(require_faBuyNLarge());
var export_faBuyNLarge = import_faBuyNLarge.definition;
export {
  export_faBuyNLarge as faBuyNLarge
};
