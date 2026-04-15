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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faRouteInterstate.js
var require_faRouteInterstate = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faRouteInterstate.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "route-interstate";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f61b";
    var svgPathData = "M85.3 192l341 0c-.8 44.3-7.9 81.8-21.9 114.4c-23.4 54.1-68.5 99.1-149 138.5c-81.5-36.2-126.1-80.9-149-135.4c-14-33.3-20.6-72-21-117.4zM276.9 7.3c-12.8-7.8-29-7.8-41.8 0c-27.5 16.8-50.2 26-73.4 30c-23.3 4-49.2 3.2-83.7-3.2C55.9 30 34.4 45 31.1 67.7C16 170.4 15.9 259.5 47.3 334.2c32.1 76.4 94.6 132.5 193 173.9c10.4 4.4 22.3 4.1 32.5-.6c95.7-44.8 157.8-100.7 190.3-175.7c32-73.9 32.9-161.2 17.8-264.1C477.6 45.1 456.1 30 434 34.1c-34.4 6.3-60.3 7.2-83.7 3.2c-23.2-4-45.9-13.2-73.4-30z";
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
    exports.faRouteInterstate = exports.definition;
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

// src/_entries/faRouteInterstate.js
var import_faRouteInterstate = __toESM(require_faRouteInterstate());
var export_faRouteInterstate = import_faRouteInterstate.definition;
export {
  export_faRouteInterstate as faRouteInterstate
};
