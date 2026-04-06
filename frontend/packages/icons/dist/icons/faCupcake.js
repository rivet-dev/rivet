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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCupcake.js
var require_faCupcake = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCupcake.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "cupcake";
    var width = 448;
    var height = 512;
    var aliases = [129473];
    var unicode = "e402";
    var svgPathData = "M240 0c-6.5 0-12.3 3.9-14.8 9.9s-1.1 12.9 3.5 17.4l6.1 6.1c3.4 3.4 5.3 7.9 5.3 12.7C240 56 232 64 222.1 64L160 64c-35.3 0-64 28.7-64 64l0 .8c-3.3 .5-6.6 1.4-9.9 2.4L50.9 143C20.5 153.2 0 181.6 0 213.6C0 254.7 33.3 288 74.4 288l299.2 0c41.1 0 74.4-33.3 74.4-74.4c0-32-20.5-60.4-50.9-70.6l-35.3-11.8c-4.1-1.4-8.2-2.3-12.5-2.8c1.7-6.7 2.6-13.8 2.6-21C352 48.1 303.9 0 244.6 0L240 0zm27.9 512l24-192-135.8 0 24 192 87.8 0zM64 320L89.3 471.9C93.2 495 113.2 512 136.7 512l11.2 0-24-192L64 320zM300.1 512l11.2 0c23.5 0 43.5-17 47.3-40.1L384 320l-59.9 0-24 192z";
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
    exports.faCupcake = exports.definition;
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

// src/_entries/faCupcake.js
var import_faCupcake = __toESM(require_faCupcake());
var export_faCupcake = import_faCupcake.definition;
export {
  export_faCupcake as faCupcake
};
