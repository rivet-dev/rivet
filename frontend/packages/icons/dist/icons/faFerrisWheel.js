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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFerrisWheel.js
var require_faFerrisWheel = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFerrisWheel.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "ferris-wheel";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e174";
    var svgPathData = "M302.7 36.9c27.7 5.9 53.5 16.9 76.4 31.9c6.3-3.1 13.4-4.8 20.9-4.8c26.5 0 48 21.5 48 48c0 7.5-1.7 14.6-4.8 20.9c15.1 22.9 26 48.7 31.9 76.4c21.1 5 36.9 24 36.9 46.7s-15.7 41.7-36.9 46.7c-5.9 27.7-16.9 53.5-31.9 76.4c3.1 6.3 4.8 13.4 4.8 20.9c0 25.6-20 46.4-45.1 47.9l-38.5-80c4.6-5.1 10.3-9.2 16.6-12c17.2-21.5 29-47.5 33.2-75.9l-81.9 0c2.4-7.6 3.7-15.6 3.7-24s-1.3-16.4-3.7-24l81.9 0c-3.9-26.3-14.3-50.4-29.4-70.9L326.9 219c-7.6-14.5-19.5-26.4-33.9-33.9l57.9-57.9c-20.5-15.1-44.6-25.4-70.9-29.4l0 81.9c-7.6-2.4-15.6-3.7-24-3.7s-16.4 1.3-24 3.7l0-81.9c-26.3 4-50.4 14.3-70.9 29.4L219 185.1c-14.5 7.6-26.4 19.5-33.9 33.9l-57.9-57.9c-15.1 20.5-25.4 44.6-29.4 70.9l81.9 0c-2.4 7.6-3.7 15.6-3.7 24s1.3 16.4 3.7 24l-81.9 0c4.3 28.5 16.1 54.4 33.2 75.9c6.4 2.8 12.1 6.9 16.7 12l-38.5 80C84 446.4 64 425.6 64 400c0-7.5 1.7-14.6 4.8-20.9c-15.1-22.9-26.1-48.7-31.9-76.4C15.7 297.7 0 278.7 0 256s15.7-41.7 36.9-46.7c5.9-27.7 16.9-53.5 31.9-76.4C65.7 126.6 64 119.5 64 112c0-26.5 21.5-48 48-48c7.5 0 14.6 1.7 20.9 4.8c22.9-15.1 48.7-26.1 76.4-31.9C214.3 15.7 233.3 0 256 0s41.7 15.7 46.7 36.9zM304 256c0 11.6-4.1 22.3-11 30.6l88.8 191.3c3.4 7.4 2.9 16.1-1.5 23s-12 11.1-20.2 11.1l-208 0c-8.2 0-15.8-4.2-20.2-11.1s-5-15.6-1.5-23L219 286.6c-6.9-8.3-11-19-11-30.6c0-26.5 21.5-48 48-48s48 21.5 48 48z";
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
    exports.faFerrisWheel = exports.definition;
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

// src/_entries/faFerrisWheel.js
var import_faFerrisWheel = __toESM(require_faFerrisWheel());
var export_faFerrisWheel = import_faFerrisWheel.definition;
export {
  export_faFerrisWheel as faFerrisWheel
};
