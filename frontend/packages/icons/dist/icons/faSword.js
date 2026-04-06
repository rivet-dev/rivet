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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSword.js
var require_faSword = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSword.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "sword";
    var width = 512;
    var height = 512;
    var aliases = [128481];
    var unicode = "f71c";
    var svgPathData = "M511.8 18.6c.8-5.1-.8-10.3-4.5-13.9S498.5-.6 493.4 .2L417.6 12.8c-16.4 2.7-31.6 10.6-43.4 22.3L166.6 242.7 269.3 345.4 476.8 137.8c11.8-11.8 19.6-27 22.3-43.4l12.6-75.8zM84.7 228.7l-16 16c-5.4 5.4-6.2 13.8-2 20.2l53.1 79.6c2.1 3.2 1.7 7.4-1 10.1L79.2 394.2c-2.1 2.1-5.3 2.9-8.2 1.9L37.1 384.8c-5.7-1.9-12.1-.4-16.4 3.9l-16 16c-6.2 6.2-6.2 16.4 0 22.6l80 80c6.2 6.2 16.4 6.2 22.6 0l16-16c4.3-4.3 5.8-10.6 3.9-16.4L115.9 441c-1-2.9-.2-6 1.9-8.2l39.6-39.6c2.7-2.7 6.9-3.1 10.1-1l79.6 53.1c6.3 4.2 14.8 3.4 20.2-2l16-16c6.2-6.2 6.2-16.4 0-22.6l-176-176c-6.2-6.2-16.4-6.2-22.6 0z";
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
    exports.faSword = exports.definition;
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

// src/_entries/faSword.js
var import_faSword = __toESM(require_faSword());
var export_faSword = import_faSword.definition;
export {
  export_faSword as faSword
};
