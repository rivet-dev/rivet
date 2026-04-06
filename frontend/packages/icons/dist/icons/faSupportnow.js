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

// src/node_modules/@fortawesome/free-brands-svg-icons/faSupportnow.js
var require_faSupportnow = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faSupportnow.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "supportnow";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e833";
    var svgPathData = "M0 191.9l203.9-.1c1.8-.6 2.5-9.5 3.2-12.2 3.3-13.5 8.4-26.4 14.8-38.7l-160.2 0 0-48.2 234.5 0 0 39c-51.2 36.1-56.4 92.6-23.6 143.9 33.1 51.7 93.4 88.1 149.5 110.5 67.8-25.2 164.9-89.9 169.4-169.6 3.1-55.3-42.2-101.8-98.7-91.8-41.5 7.4-62.1 46.7-54.8 86.4l-48.2 0c-7.3-56.6 22.8-112.6 78.4-130.3 78.7-25.1 159.5 28 170.4 108.8 15.2 112.3-95.5 199.9-188.1 237.8-4.4 1.8-24.4 10.1-27.5 10.1l-280.3 0 0-48.2 178.5 0-38.1-28.9-23.2-22.1-198.3 0 0-48.2 163.1 0c-7.1-11.6-12.3-24.3-16.1-37.4-.7-2.5-2.1-12-3.8-12.6l-204.9-.1 0-48.2 0 0z";
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
    exports.faSupportnow = exports.definition;
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

// src/_entries/faSupportnow.js
var import_faSupportnow = __toESM(require_faSupportnow());
var export_faSupportnow = import_faSupportnow.definition;
export {
  export_faSupportnow as faSupportnow
};
