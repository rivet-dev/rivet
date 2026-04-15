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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faArrowsRepeat1.js
var require_faArrowsRepeat1 = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faArrowsRepeat1.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "arrows-repeat-1";
    var width = 512;
    var height = 512;
    var aliases = ["repeat-1-alt"];
    var unicode = "f366";
    var svgPathData = "M21.9 254.4C5.1 248.8-3.9 230.6 1.6 213.9C25.1 143.5 91 96 165.2 96l173.6 0L297.4 54.6c-12.5-12.5-12.5-32.8 0-45.3s32.8-12.5 45.3 0l96 96c9.2 9.2 11.9 22.9 6.9 34.9s-16.6 19.8-29.6 19.8l-250.8 0c-46.7 0-88.1 29.9-102.8 74.1c-5.6 16.8-23.7 25.8-40.5 20.2zm468.2 3.3c16.8 5.6 25.8 23.7 20.2 40.5C486.9 368.5 421 416 346.8 416l-173.6 0 41.4 41.4c12.5 12.5 12.5 32.8 0 45.3s-32.8 12.5-45.3 0l-96-96c-9.2-9.2-11.9-22.9-6.9-34.9s16.6-19.8 29.6-19.8l250.8 0c46.7 0 88.1-29.9 102.8-74.1c5.6-16.8 23.7-25.8 40.5-20.2zM288 216l0 80c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-48c-10 0-19.4-6.4-22.8-16.4c-4.2-12.6 2.6-26.2 15.2-30.4l24-8c7.3-2.4 15.4-1.2 21.6 3.3s10 11.8 10 19.5z";
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
    exports.faArrowsRepeat1 = exports.definition;
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

// src/_entries/faRepeat1Alt.js
var import_faArrowsRepeat1 = __toESM(require_faArrowsRepeat1());
var export_faRepeat1Alt = import_faArrowsRepeat1.definition;
export {
  export_faRepeat1Alt as faRepeat1Alt
};
