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

// src/node_modules/@fortawesome/free-brands-svg-icons/faCreativeCommonsRemix.js
var require_faCreativeCommonsRemix = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faCreativeCommonsRemix.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "creative-commons-remix";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f4ee";
    var svgPathData = "M255.6 8C397.4 8 504 118.1 504 256 504 403.1 385.5 504 255.6 504 121.6 504 8 394.5 8 256 8 123.1 112.7 8 255.6 8zm.8 44.7c-118.2 0-203.7 97.9-203.7 203.3 0 109.8 91.2 202.8 203.7 202.8 103.2 0 202.8-81.1 202.8-202.8 .1-113.8-90.2-203.3-202.8-203.3zM418.1 260.4l4.9 2.2 0 70c-7.2 3.6-63.4 27.5-67.3 28.8-6.5-1.8-113.7-46.8-137.3-56.2l-64.2 26.6-63.3-27.5 0-63.8 59.3-24.8c-.7-.7-.4 5-.4-70.4l67.3-29.7 151.9 62.9 0 61.6 49.1 20.3zm-70.4 81.5l0-43.8-.4 0 0-1.8-113.8-46.5 0 45.2 113.8 46.9 0-.4 .4 .4zm7.5-57.6l39.9-16.4-36.8-15.5-39 16.4 35.9 15.5zm52.3 38.1l0-43-44.3 18.6 0 43.4 44.3-19z";
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
    exports.faCreativeCommonsRemix = exports.definition;
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

// src/_entries/faCreativeCommonsRemix.js
var import_faCreativeCommonsRemix = __toESM(require_faCreativeCommonsRemix());
var export_faCreativeCommonsRemix = import_faCreativeCommonsRemix.definition;
export {
  export_faCreativeCommonsRemix as faCreativeCommonsRemix
};
