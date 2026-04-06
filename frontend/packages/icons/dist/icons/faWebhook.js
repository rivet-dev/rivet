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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faWebhook.js
var require_faWebhook = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faWebhook.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "webhook";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e5d5";
    var svgPathData = "M306 50.1c-43.1-9.9-86 16.9-95.9 60c-7.9 34.1 7.4 68.2 35.5 85.9c5.4 3.4 9.2 8.8 10.7 15s.3 12.7-3 18.1L170.4 361.5c3.6 6.7 5.6 14.4 5.6 22.5c0 26.5-21.5 48-48 48s-48-21.5-48-48s21.5-48 48-48c.6 0 1.1 0 1.7 0L201 221.9c-32.5-30.2-48.4-76.4-37.7-122.7C179.2 30.3 247.9-12.6 316.8 3.3c65.9 15.2 108 78.7 97.7 144.4c-2.1 13.1-14.3 22-27.4 20s-22-14.3-20-27.4c6.4-41.1-19.9-80.7-61.1-90.2zM289.7 176c-.6 0-1.1 0-1.7 0c-26.5 0-48-21.5-48-48s21.5-48 48-48s48 21.5 48 48c0 8.1-2 15.8-5.6 22.5l71.3 114.1c45.8-17.7 99.8-8.2 136.8 28.9c50 50 50 131 0 181c-43 43-109 49-158.4 18c-11.2-7-14.6-21.8-7.6-33.1s21.8-14.6 33.1-7.6c30.9 19.3 72.1 15.5 99-11.3c31.2-31.2 31.2-81.9 0-113.1c-26.8-26.8-68.1-30.6-99-11.3c-5.4 3.4-11.9 4.5-18.1 3s-11.6-5.3-15-10.7L289.7 176zM448 432c-17.8 0-33.3-9.7-41.6-24l-152.7 0c-9.2 48.3-46 89-97 100.7c-68.9 15.9-137.6-27-153.5-95.9C-11 351 22.1 289.4 78.7 265.8c12.2-5.1 26.3 .7 31.4 12.9s-.7 26.3-12.9 31.4c-35.4 14.7-56 53.3-47.1 91.8c9.9 43 52.9 69.9 95.9 60c37-8.5 62.1-41.5 62-77.9c0-6.4 2.5-12.5 7-17s10.6-7 17-7l174.4 0c8.3-14.3 23.8-24 41.6-24c26.5 0 48 21.5 48 48s-21.5 48-48 48z";
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
    exports.faWebhook = exports.definition;
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

// src/_entries/faWebhook.js
var import_faWebhook = __toESM(require_faWebhook());
var export_faWebhook = import_faWebhook.definition;
export {
  export_faWebhook as faWebhook
};
