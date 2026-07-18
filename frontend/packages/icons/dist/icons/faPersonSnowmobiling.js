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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPersonSnowmobiling.js
var require_faPersonSnowmobiling = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPersonSnowmobiling.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "person-snowmobiling";
    var width = 640;
    var height = 512;
    var aliases = ["snowmobile"];
    var unicode = "f7d1";
    var svgPathData = "M240 96a48 48 0 1 0 0-96 48 48 0 1 0 0 96zM373.5 85.3c-5.9-11.9-20.3-16.7-32.2-10.7s-16.7 20.3-10.7 32.2l37.4 74.7L360 192l-56 0-61.7-55.6c-11.8-10.6-27-16.4-42.8-16.4l-.9 0c-23.7 0-45.5 13.1-56.6 34.1l-35.3 66.6c-11.4 21.6-9.4 47.8 5.3 67.4l-12.2 0c-12.1 0-23.2 6.8-28.6 17.7l-32 64c-2.5 5-3.6 10.2-3.4 15.2c4-.6 8.1-.9 12.3-.9l224 0c26.2 0 49.4 12.6 64 32l114 0 36 48-46 0c-13.3 0-24 10.7-24 24s10.7 24 24 24l136 0c6.4 0 12.5-2.5 17-7l40-40c9.4-9.4 9.4-24.6 0-33.9s-24.6-9.4-33.9 0l-33 33L546 464l-46-61.3 68.9-45.9c4.5-3 7.1-8 7.1-13.3l0-77.5c0-6.1-3.4-11.6-8.8-14.3L419.8 177.9 373.5 85.3zM298.5 256l13.5 0-24 32-24 0-72-32 32-48 29.3 29.3c12 12 28.3 18.7 45.3 18.7zM48 416c-26.5 0-48 21.5-48 48s21.5 48 48 48l224 0c26.5 0 48-21.5 48-48s-21.5-48-48-48L48 416z";
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
    exports.faPersonSnowmobiling = exports.definition;
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

// src/_entries/faPersonSnowmobiling.js
var import_faPersonSnowmobiling = __toESM(require_faPersonSnowmobiling());
var export_faPersonSnowmobiling = import_faPersonSnowmobiling.definition;
export {
  export_faPersonSnowmobiling as faPersonSnowmobiling
};
