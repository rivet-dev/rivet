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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSpiderWeb.js
var require_faSpiderWeb = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSpiderWeb.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "spider-web";
    var width = 576;
    var height = 512;
    var aliases = [128376];
    var unicode = "f719";
    var svgPathData = "M133.8 22.2c4.8-12.7 18-20.2 31.4-17.7l69 12.9c35.6 6.7 72.1 6.7 107.6 0l69-12.9c13.4-2.5 26.6 5 31.4 17.7l28.9 77.2c11.5 30.7 28.1 59.2 49.1 84.4l45.3 54.3c8.7 10.4 8.7 25.5 0 35.9l-45.3 54.3c-21 25.2-37.6 53.7-49.1 84.4l-28.9 77.2c-4.8 12.7-18 20.2-31.4 17.7l-69-12.9c-35.6-6.7-72.1-6.7-107.6 0l-69 12.9c-13.4 2.5-26.6-5-31.4-17.7l-28.9-77.2C93.3 382 76.7 353.4 55.7 328.2L10.5 273.9c-8.7-10.4-8.7-25.5 0-35.9l45.3-54.3c21-25.2 37.6-53.7 49.1-84.4l28.9-77.2zm33.3 70.6L157.3 119c-13.7 36.6-33.5 70.6-58.5 100.6L88.4 232l44.6 0c23.5-27.6 41.7-59.2 53.7-93.4l2.5-7L167.1 92.8zm42.1-23.1l21.4 37.4c37.7 9.5 77.2 9.5 114.9 0l21.4-37.4-14.7 2.8c-42.4 7.9-85.9 7.9-128.3 0l-14.7-2.8zM408.9 92.8l-22.2 38.8 2.5 7c12.1 34.2 30.3 65.8 53.8 93.4l44.6 0-10.3-12.4c-25-30-44.8-64-58.5-100.6l-9.8-26.2zM487.6 280L443 280c-23.6 28.1-41.8 60.4-53.6 95.3l-2.1 6.1 21.6 37.9 9.8-26.2c13.7-36.6 33.5-70.6 58.5-100.6L487.6 280zM366.8 442.3l-19.5-34.2c-37.9-11.7-78.3-12.8-116.7-3.2l-21.3 37.4 14.7-2.7c42.4-7.9 85.9-7.9 128.3 0l14.7 2.7zM167.1 419.2l22.2-38.8-2.5-7c-12.1-34.2-30.3-65.8-53.7-93.4l-44.6 0 10.3 12.4c25 30 44.8 64 58.5 100.6l9.8 26.2zM193.2 280c10.1 15 18.9 30.7 26.5 47.1L246.6 280l-53.4 0zm67.9 71.4c18.1-1.5 36.3-1.2 54.3 1L288 304.4l-26.9 47zm95.5-23.7c7.6-16.6 16.4-32.5 26.5-47.7l-53.8 0 27.3 47.7zM382.8 232c-10.1-15-18.9-30.7-26.5-47.1L329.4 232l53.4 0zm-68.1-71c-17.7 1.7-35.6 1.7-53.3 0L288 207.6 314.7 161zm-94.9 23.9c-7.6 16.4-16.5 32.1-26.5 47.1l53.4 0-26.9-47.1z";
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
    exports.faSpiderWeb = exports.definition;
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

// src/_entries/faSpiderWeb.js
var import_faSpiderWeb = __toESM(require_faSpiderWeb());
var export_faSpiderWeb = import_faSpiderWeb.definition;
export {
  export_faSpiderWeb as faSpiderWeb
};
