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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFireSmoke.js
var require_faFireSmoke = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFireSmoke.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "fire-smoke";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f74b";
    var svgPathData = "M283.7 0c-3.6 0-7.2 1.2-10 3.7c-33.2 29.7-61.4 63.4-81.4 95.8c-19.7 31.9-32.4 66.2-32.4 92.6c0 30 8.2 58.3 22.7 82.4c4.6 2.6 9 5.4 13.3 8.5c13.7-16.8 30.7-30.8 50.1-40.9c-3.9-9.4-6-20.2-6-32.1c0-24 15-45 45-82c4 5 62 79 62 79l36-42c3 4 5 8 7 12c11.3 20.7 12.3 44.9 4.2 65.2c19.3 10.1 36.2 24 49.9 40.8c4.2-3 8.5-5.8 12.9-8.3c14.6-24.1 23-52.4 23-82.5c0-20.2-9.6-50.9-24.2-79C441.1 84.7 420.2 54.7 395.5 32c-5.6-5.1-14.4-5.2-20 0c-9.6 8.8-18.6 19.6-26.5 29.5c-17.3-20.7-35.8-39.9-55.5-57.7c-2.8-2.5-6.3-3.7-9.8-3.8zM0 400c0 61.9 50.1 112 112 112l208 0 208 0c61.9 0 112-50.1 112-112s-50.1-112-112-112c-37 0-69.9 18-90.3 45.7C418.2 288 372.8 256 320 256s-98.2 32-117.7 77.7C181.9 306 149 288 112 288C50.1 288 0 338.1 0 400z";
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
    exports.faFireSmoke = exports.definition;
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

// src/_entries/faFireSmoke.js
var import_faFireSmoke = __toESM(require_faFireSmoke());
var export_faFireSmoke = import_faFireSmoke.definition;
export {
  export_faFireSmoke as faFireSmoke
};
