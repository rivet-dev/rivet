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

// src/node_modules/@fortawesome/free-brands-svg-icons/faSquareWhatsapp.js
var require_faSquareWhatsapp = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faSquareWhatsapp.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "square-whatsapp";
    var width = 448;
    var height = 512;
    var aliases = ["whatsapp-square"];
    var unicode = "f40c";
    var svgPathData = "M92.1 254.6c0 24.9 7 49.2 20.2 70.1l3.1 5-13.3 48.6 49.9-13.1 4.8 2.9c20.2 12 43.4 18.4 67.1 18.4l.1 0c72.6 0 133.3-59.1 133.3-131.8 0-35.2-15.2-68.3-40.1-93.2-25-25-58-38.7-93.2-38.7-72.7 0-131.8 59.1-131.9 131.8zM274.8 330c-12.6 1.9-22.4 .9-47.5-9.9-36.8-15.9-61.8-51.5-66.9-58.7-.4-.6-.7-.9-.8-1.1-2-2.6-16.2-21.5-16.2-41 0-18.4 9-27.9 13.2-32.3 .3-.3 .5-.5 .7-.8 3.6-4 7.9-5 10.6-5 2.6 0 5.3 0 7.6 .1l.8 0c2.3 0 5.2 0 8.1 6.8 1.2 2.9 3 7.3 4.9 11.8 3.3 8 6.7 16.3 7.3 17.6 1 2 1.7 4.3 .3 6.9-3.4 6.8-6.9 10.4-9.3 13-3.1 3.2-4.5 4.7-2.3 8.6 15.3 26.3 30.6 35.4 53.9 47.1 4 2 6.3 1.7 8.6-1 2.3-2.6 9.9-11.6 12.5-15.5 2.6-4 5.3-3.3 8.9-2s23.1 10.9 27.1 12.9c.8 .4 1.5 .7 2.1 1 2.8 1.4 4.7 2.3 5.5 3.6 .9 1.9 .9 9.9-2.4 19.1-3.3 9.3-19.1 17.7-26.7 18.8zM448 96c0-35.3-28.7-64-64-64L64 32C28.7 32 0 60.7 0 96L0 416c0 35.3 28.7 64 64 64l320 0c35.3 0 64-28.7 64-64l0-320zM148.1 393.9L64 416 86.5 333.8c-13.9-24-21.2-51.3-21.2-79.3 .1-87.4 71.2-158.5 158.6-158.5 42.4 0 82.2 16.5 112.2 46.5 29.9 30 47.9 69.8 47.9 112.2 0 87.4-72.7 158.5-160.1 158.5-26.6 0-52.7-6.7-75.8-19.3z";
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
    exports.faSquareWhatsapp = exports.definition;
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

// src/_entries/faSquareWhatsapp.js
var import_faSquareWhatsapp = __toESM(require_faSquareWhatsapp());
var export_faSquareWhatsapp = import_faSquareWhatsapp.definition;
export {
  export_faSquareWhatsapp as faSquareWhatsapp
};
