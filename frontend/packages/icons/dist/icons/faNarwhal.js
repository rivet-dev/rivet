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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faNarwhal.js
var require_faNarwhal = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faNarwhal.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "narwhal";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f6fe";
    var svgPathData = "M639.3 20.7c2.4-7.8-1.4-16.1-8.9-19.4s-16.2-.4-20.2 6.6L516.7 168.3c-16.9-5.4-34.9-8.3-53.6-8.3c-50.9 0-100.7 17.5-140.2 49.6L154.1 346.8c-10.5 8.5-26.1 1.1-26.1-12.4l0-81.3 29.9-20.9c21.4-15 34.1-39.4 34.1-65.5l0-54.7c0-6-3.3-11.4-8.6-14.2s-11.7-2.3-16.6 1.1L96 148.5 25.2 98.9c-4.9-3.4-11.3-3.8-16.6-1.1S0 106 0 112l0 54.7c0 26.1 12.7 50.6 34.1 65.5L64 253.1 64 352c0 88.4 71.6 160 160 160l240 0c97.2 0 176-78.8 176-176c0-51.4-22.3-97.7-57.8-129.9l57-185.4zM496 280a24 24 0 1 1 0 48 24 24 0 1 1 0-48z";
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
    exports.faNarwhal = exports.definition;
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

// src/_entries/faNarwhal.js
var import_faNarwhal = __toESM(require_faNarwhal());
var export_faNarwhal = import_faNarwhal.definition;
export {
  export_faNarwhal as faNarwhal
};
