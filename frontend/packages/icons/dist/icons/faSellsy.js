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

// src/node_modules/@fortawesome/free-brands-svg-icons/faSellsy.js
var require_faSellsy = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faSellsy.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "sellsy";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f213";
    var svgPathData = "M540 237.3c3.1-12.3 4.3-24.8 4.3-37.4 0-92.5-75.4-167.9-167.9-167.9-77.2 0-144.6 53-163 127.8-15.3-13.2-34.9-20.5-55.2-20.5-46.3 0-84 37.7-84 84 0 7.4 .9 15 3.1 22.4-42.9 20.2-70.8 63.7-70.8 111.2 0 68 55.5 123.2 123.2 123.2l381.2 0c67.7 0 123.2-55.2 123.2-123.2 0-56.4-38.9-106-94.1-119.5zM200.2 401.6c0 8.3-7 15.3-15.3 15.3l-30.9 0c-8.3 0-15.3-7-15.3-15.3l0-110.9c0-8.3 7-15.3 15.3-15.3l30.9 0c8.3 0 15.3 7 15.3 15.3l0 110.9zm89.5 0c0 8.3-7 15.3-15.3 15.3l-30.9 0c-8.3 0-15.3-7-15.3-15.3l0-131.5c0-8.3 7-15.3 15.3-15.3l30.9 0c8.3 0 15.3 7 15.3 15.3l0 131.5zm89.5 0c0 8.3-7 15.3-15.3 15.3l-31 0c-8.3 0-15.3-7-15.3-15.3l0-162.7c0-8.3 7-15.3 15.3-15.3l31 0c8.3 0 15.3 7 15.3 15.3l0 162.7zm87 0c0 8.3-7 15.3-15.3 15.3l-28.5 0c-8.3 0-15.3-7-15.3-15.3l0-224.6c0-8.6 7-15.6 15.3-15.6l28.5 0c8.3 0 15.3 7 15.3 15.6l0 224.6z";
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
    exports.faSellsy = exports.definition;
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

// src/_entries/faSellsy.js
var import_faSellsy = __toESM(require_faSellsy());
var export_faSellsy = import_faSellsy.definition;
export {
  export_faSellsy as faSellsy
};
