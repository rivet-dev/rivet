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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faViolin.js
var require_faViolin = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faViolin.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "violin";
    var width = 640;
    var height = 512;
    var aliases = [127931];
    var unicode = "f8ed";
    var svgPathData = "M630.6 54.6c12.5-12.5 12.5-32.8 0-45.3s-32.8-12.5-45.3 0L417 177.7C398 166.5 375.7 160 352 160c-20.8 0-40.4 5-57.8 13.8c-6.7 3.4-9.1 11.4-7.7 18.8c1 5 1.5 10.2 1.5 15.4c0 44.2-35.8 80-80 80c-11.6 0-22.7-2.5-32.7-7c-7.4-3.3-16.6-2.4-21.3 4.3C137.6 308.7 128 337.2 128 368c0 15 2.3 29.5 6.6 43.1l44.6-33.5c19.1-14.3 45.9-12.4 62.7 4.5l16 16c16.9 16.9 18.8 43.6 4.5 62.7l-33.5 44.6c13.6 4.3 28.1 6.6 43.1 6.6c30.8 0 59.3-9.6 82.6-26.1c6.7-4.7 7.6-13.8 4.3-21.3c-4.5-10-7-21-7-32.7c0-44.2 35.8-80 80-80c5.3 0 10.5 .5 15.4 1.5c7.4 1.4 15.4-1 18.8-7.7c8.8-17.4 13.8-37 13.8-57.8c0-23.7-6.5-46-17.7-65L630.6 54.6zM520 48a24 24 0 1 0 0-48 24 24 0 1 0 0 48zM345 41c9.4-9.4 9.4-24.6 0-33.9s-24.6-9.4-33.9 0L7 311c-9.4 9.4-9.4 24.6 0 33.9s24.6 9.4 33.9 0L345 41zM472 96a24 24 0 1 0 0-48 24 24 0 1 0 0 48zm120 72a24 24 0 1 0 -48 0 24 24 0 1 0 48 0zm48-48a24 24 0 1 0 -48 0 24 24 0 1 0 48 0zM198.4 403.2l-64 48c-3.7 2.8-6 7-6.4 11.7s1.4 9.2 4.6 12.4l32 32c3.3 3.3 7.8 5 12.4 4.6s8.9-2.6 11.7-6.4l48-64c4.8-6.4 4.1-15.3-1.5-20.9l-16-16c-5.6-5.6-14.5-6.3-20.9-1.5z";
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
    exports.faViolin = exports.definition;
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

// src/_entries/faViolin.js
var import_faViolin = __toESM(require_faViolin());
var export_faViolin = import_faViolin.definition;
export {
  export_faViolin as faViolin
};
