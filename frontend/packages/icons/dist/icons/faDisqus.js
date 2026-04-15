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

// src/node_modules/@fortawesome/free-brands-svg-icons/faDisqus.js
var require_faDisqus = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faDisqus.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "disqus";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e7d5";
    var svgPathData = "M290.2 512c-62.6 0-119.9-22.9-164.2-60.8L17 466.1 59.1 362.2c-14.7-32.4-22.9-68.3-22.9-106.2 0-141.4 113.7-256 254-256 140.3 0 254 114.6 254 256 0 141.4-113.7 256-254 256zM428.9 255.3l0-.7C428.9 180.7 376.8 128 287 128l-97 0 0 256 95.6 0c90.5 0 143.4-54.9 143.4-128.7zM288 321.1l-28.4 0 0-130.2 28.4 0c41.7 0 69.3 23.8 69.3 64.7l0 .7c0 41.3-27.7 64.7-69.3 64.7z";
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
    exports.faDisqus = exports.definition;
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

// src/_entries/faDisqus.js
var import_faDisqus = __toESM(require_faDisqus());
var export_faDisqus = import_faDisqus.definition;
export {
  export_faDisqus as faDisqus
};
