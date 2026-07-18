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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faAwardSimple.js
var require_faAwardSimple = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faAwardSimple.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "award-simple";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "e0ab";
    var svgPathData = "M173.3 5.3c11.5-7 26-7 37.5 0l17.9 10.9c5.9 3.6 12.7 5.4 19.6 5.2l20.9-.5c13.5-.3 26 6.9 32.5 18.7l10 18.4c3.3 6.1 8.3 11 14.3 14.3l18.4 10c11.8 6.5 19.1 19 18.7 32.5l-.5 20.9c-.2 6.9 1.7 13.7 5.2 19.6l10.9 17.9c7 11.5 7 26 0 37.5l-10.9 17.9c-3.6 5.9-5.4 12.7-5.2 19.6l.5 20.9c.3 13.5-6.9 26-18.7 32.5l-18.4 10c-6.1 3.3-11 8.3-14.3 14.3l-10 18.4c-6.5 11.8-19 19.1-32.5 18.7l-20.9-.5c-6.9-.2-13.7 1.7-19.6 5.2l-17.9 10.9c-11.5 7-26 7-37.5 0l-17.9-10.9c-5.9-3.6-12.7-5.4-19.6-5.2l-20.9 .5c-13.5 .3-26-6.9-32.5-18.7L72.4 326c-3.3-6.1-8.3-11-14.3-14.3l-18.4-10c-11.8-6.5-19.1-19-18.7-32.5l.5-20.9c.2-6.9-1.7-13.7-5.2-19.6L5.3 210.7c-7-11.5-7-26 0-37.5l10.9-17.9c3.6-5.9 5.4-12.7 5.2-19.6l-.5-20.9c-.3-13.5 6.9-26 18.7-32.5L58 72.4c6.1-3.3 11-8.3 14.3-14.3l10-18.4c6.5-11.8 19-19.1 32.5-18.7l20.9 .5c6.9 .2 13.7-1.7 19.6-5.2L173.3 5.3zM272 192a80 80 0 1 0 -160 0 80 80 0 1 0 160 0zM64.4 492.4L88 390c8.9 3.7 18.6 5.6 28.6 5.4l20.7-.5c.2 0 .4 .1 .6 .2l17.6 10.7c22.4 13.6 50.5 13.6 72.9 0L246.1 395c.2-.1 .4-.2 .6-.2l20.7 .5c10 .2 19.7-1.7 28.6-5.4l23.6 102.4c1.3 5.5-.5 11.3-4.6 15.2s-10 5.3-15.4 3.8L192 480.6 84.4 511.4c-5.4 1.6-11.3 .1-15.4-3.8s-5.8-9.7-4.6-15.2z";
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
    exports.faAwardSimple = exports.definition;
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

// src/_entries/faAwardSimple.js
var import_faAwardSimple = __toESM(require_faAwardSimple());
var export_faAwardSimple = import_faAwardSimple.definition;
export {
  export_faAwardSimple as faAwardSimple
};
