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

// src/node_modules/@fortawesome/free-solid-svg-icons/faPersonCircleQuestion.js
var require_faPersonCircleQuestion = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faPersonCircleQuestion.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "person-circle-question";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e542";
    var svgPathData = "M280 24a56 56 0 1 0 -112 0 56 56 0 1 0 112 0zm24 212.7L341 286.6c12.8-17.5 28.5-32.7 46.3-45l-56.2-75.7C306 132 266.3 112 224 112s-82 20-107.2 53.9l-70.5 95c-10.5 14.2-7.6 34.2 6.6 44.8s34.2 7.6 44.8-6.6L144 236.7 144 512c0 17.7 14.3 32 32 32s32-14.3 32-32l0-160c0-8.8 7.2-16 16-16s16 7.2 16 16l0 160c0 17.7 14.3 32 32 32s32-14.3 32-32l0-275.3zM496 544a144 144 0 1 0 0-288 144 144 0 1 0 0 288zm0-100a20 20 0 1 1 0 40 20 20 0 1 1 0-40zm0-100c-11.6 0-21.3 8.2-23.5 19.2-1.8 8.7-10.2 14.3-18.9 12.5s-14.3-10.2-12.5-18.9c5.2-25.6 27.8-44.8 54.9-44.8 30.9 0 56 25.1 56 56 0 19.8-11.7 37.8-29.8 45.9l-10.4 4.6c-1.2 7.7-7.8 13.5-15.8 13.5-8.8 0-16-7.2-16-16 0-11.2 6.6-21.3 16.8-25.9l12.4-5.5c6.6-2.9 10.8-9.4 10.8-16.6 0-13.3-10.7-24-24-24z";
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
    exports.faPersonCircleQuestion = exports.definition;
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

// src/_entries/faPersonCircleQuestion.js
var import_faPersonCircleQuestion = __toESM(require_faPersonCircleQuestion());
var export_faPersonCircleQuestion = import_faPersonCircleQuestion.definition;
export {
  export_faPersonCircleQuestion as faPersonCircleQuestion
};
